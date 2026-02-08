import katex from "katex";

interface ViewerState {
  scale: number;
  offsetX: number;
  offsetY: number;
  isDragging: boolean;
  lastMouseX: number;
  lastMouseY: number;
}

class MathWindowController {
  #container: HTMLElement | null = null;
  #wrapper: HTMLElement | null = null;
  #contentContainer: HTMLElement | null = null;
  #maxZoom: number = 100.0;
  #state: ViewerState = {
    scale: 1.0,
    offsetX: 0,
    offsetY: 0,
    isDragging: false,
    lastMouseX: 0,
    lastMouseY: 0,
  };

  async init(source: string, mathId: string): Promise<void> {
    this.#container = document.getElementById("math-window-canvas");
    this.#wrapper = document.getElementById("math-content-wrapper");
    this.#contentContainer = document.getElementById("math-content-container");

    if (!this.#container || !this.#wrapper || !this.#contentContainer) {
      throw new Error("Viewer container not found");
    }

    // Render KaTeX (synchronous)
    this.#renderMath(source, mathId);

    // Setup event listeners
    this.#setupEventListeners();

    // Listen for theme changes
    document.addEventListener("arto:theme-changed", ((event: CustomEvent) => {
      this.setTheme(event.detail);
    }) as EventListener);

    // Initial fit to window after browser layout
    setTimeout(() => this.#fitToWindow(), 100);
  }

  setTheme(theme: string): void {
    // Only update body theme attribute (KaTeX is monochrome, no re-render needed)
    document.body.setAttribute("data-theme", theme);
    console.log("Theme changed to:", theme);
  }

  #renderMath(source: string, mathId: string): void {
    if (!this.#contentContainer) return;

    try {
      const html = katex.renderToString(source, {
        displayMode: true,
        throwOnError: false,
      });
      this.#contentContainer.innerHTML = html;
      this.#contentContainer.setAttribute("data-math-source", source);
      this.#contentContainer.setAttribute("data-math-id", mathId);
    } catch (error) {
      console.error("Failed to render math:", error);
      if (this.#contentContainer) {
        this.#contentContainer.innerHTML = "";

        const wrapper = document.createElement("div");
        wrapper.style.color = "red";
        wrapper.style.padding = "2rem";

        const strong = document.createElement("strong");
        strong.textContent = "Rendering Error:";

        const lineBreak = document.createElement("br");

        const pre = document.createElement("pre");
        pre.style.whiteSpace = "pre-wrap";
        const errorMessage = error instanceof Error ? error.message : String(error);
        pre.textContent = errorMessage;

        wrapper.appendChild(strong);
        wrapper.appendChild(lineBreak);
        wrapper.appendChild(pre);

        this.#contentContainer.appendChild(wrapper);
      }
    }
  }

  #setupEventListeners(): void {
    if (!this.#container) return;

    // Keyboard shortcuts
    document.addEventListener("keydown", this.#handleKeyDown.bind(this));

    // Mouse events for dragging
    this.#container.addEventListener("mousedown", this.#handleMouseDown.bind(this));
    document.addEventListener("mousemove", this.#handleMouseMove.bind(this));
    document.addEventListener("mouseup", this.#handleMouseUp.bind(this));

    // Scroll events
    this.#container.addEventListener("wheel", this.#handleWheel.bind(this), { passive: false });

    // Double-click to fit
    this.#container.addEventListener("dblclick", this.#handleDoubleClick.bind(this));
  }

  #handleKeyDown(event: KeyboardEvent): void {
    const isCmdOrCtrl = event.metaKey || event.ctrlKey;

    if (isCmdOrCtrl) {
      if (event.key === "=" || event.key === "+") {
        event.preventDefault();
        this.#zoom(0.1);
      } else if (event.key === "-") {
        event.preventDefault();
        this.#zoom(-0.1);
      } else if (event.key === "0") {
        event.preventDefault();
        this.#fitToWindow();
      }
    }
  }

  #handleMouseDown(event: MouseEvent): void {
    if (event.button === 0) {
      // Left click
      this.#state.isDragging = true;
      this.#state.lastMouseX = event.clientX;
      this.#state.lastMouseY = event.clientY;
      if (this.#container) {
        this.#container.style.cursor = "grabbing";
      }
    }
  }

  #handleMouseMove(event: MouseEvent): void {
    if (this.#state.isDragging) {
      const dx = event.clientX - this.#state.lastMouseX;
      const dy = event.clientY - this.#state.lastMouseY;

      // When using CSS zoom, translate values are in the zoomed coordinate space
      // So we need to divide by scale to get the correct movement
      this.#state.offsetX += dx;
      this.#state.offsetY += dy;

      this.#state.lastMouseX = event.clientX;
      this.#state.lastMouseY = event.clientY;

      this.#updateTransform();
    }
  }

  #handleMouseUp(): void {
    this.#state.isDragging = false;
    if (this.#container) {
      this.#container.style.cursor = "grab";
    }
  }

  #handleWheel(event: WheelEvent): void {
    // Always zoom with scroll (no modifier key needed)
    event.preventDefault();

    // Exponential zoom: scale relative to current zoom level
    // This provides natural feel - same perceived change at any zoom level
    const deltaScale = this.#getDeltaModeScale(event.deltaMode);
    const deltaY = event.deltaY * deltaScale;
    const ZOOM_SCALE = 0.01;
    const zoomFactor = Math.exp(-deltaY * ZOOM_SCALE);

    const oldScale = this.#state.scale;
    const newScale = Math.max(0.1, Math.min(this.#maxZoom, oldScale * zoomFactor));

    if (newScale !== oldScale) {
      // Get mouse position relative to container
      const rect = this.#container!.getBoundingClientRect();
      const mouseX = event.clientX - rect.left;
      const mouseY = event.clientY - rect.top;

      // Point in diagram space (unaffected by wrapper transform)
      const diagramX = (mouseX - this.#state.offsetX) / oldScale;
      const diagramY = (mouseY - this.#state.offsetY) / oldScale;

      // New offset to keep the diagram point at the mouse position
      this.#state.offsetX = mouseX - diagramX * newScale;
      this.#state.offsetY = mouseY - diagramY * newScale;
      this.#state.scale = newScale;

      this.#updateTransform();
      this.#updateZoomDisplay();
    }
  }

  #handleDoubleClick(): void {
    this.#fitToWindow();
  }

  #getDeltaModeScale(deltaMode: number): number {
    switch (deltaMode) {
      case WheelEvent.DOM_DELTA_PIXEL:
        return 1;
      case WheelEvent.DOM_DELTA_LINE:
        return 10;
      case WheelEvent.DOM_DELTA_PAGE:
        return 20;
      default:
        return 1;
    }
  }

  #zoom(delta: number): void {
    const newScale = Math.max(0.1, Math.min(this.#maxZoom, this.#state.scale + delta));

    // Zoom to center
    if (this.#container) {
      const centerX = this.#container.clientWidth / 2;
      const centerY = this.#container.clientHeight / 2;

      // With CSS zoom, we need to adjust for the zoom factor
      const oldScale = this.#state.scale;
      const scaleRatio = newScale / oldScale;

      // Adjust offset: the point that was at centerX/Y should stay at centerX/Y
      this.#state.offsetX = centerX - (centerX - this.#state.offsetX) * scaleRatio;
      this.#state.offsetY = centerY - (centerY - this.#state.offsetY) * scaleRatio;
    }

    this.#state.scale = newScale;
    this.#updateTransform();
    this.#updateZoomDisplay();
  }

  #fitToWindow(): void {
    if (!this.#container || !this.#contentContainer) return;

    // Use scrollWidth/scrollHeight for HTML content dimensions
    const contentWidth = this.#contentContainer.scrollWidth;
    const contentHeight = this.#contentContainer.scrollHeight;
    if (contentWidth === 0 || contentHeight === 0) return;

    const padding = 40; // padding on each side

    // Available space in the canvas
    const availableWidth = this.#container.clientWidth - padding * 2;
    const availableHeight = this.#container.clientHeight - padding * 2;

    // Calculate scale to fit (allow up to max zoom)
    const scaleX = availableWidth / contentWidth;
    const scaleY = availableHeight / contentHeight;
    const scale = Math.min(scaleX, scaleY, this.#maxZoom);

    this.#state.scale = scale;

    // Center the content in the container
    const scaledWidth = contentWidth * scale;
    const scaledHeight = contentHeight * scale;

    // Center horizontally and vertically
    this.#state.offsetX = (this.#container.clientWidth - scaledWidth) / 2;
    this.#state.offsetY = (this.#container.clientHeight - scaledHeight) / 2;

    this.#updateTransform(false); // No animation for instant fit
    this.#updateZoomDisplay();
  }

  #updateTransform(animate = false): void {
    if (!this.#wrapper || !this.#contentContainer) return;

    if (animate) {
      this.#wrapper.style.transition = "transform 0.3s ease-out";
      this.#contentContainer.style.transition = "zoom 0.3s ease-out";
    } else {
      this.#wrapper.style.transition = "none";
      this.#contentContainer.style.transition = "none";
    }

    // Separate zoom and translate to avoid coordinate space issues
    // wrapper handles position (translate)
    this.#wrapper.style.transform = `translate(${this.#state.offsetX}px, ${this.#state.offsetY}px)`;
    // inner container handles zoom
    this.#contentContainer.style.zoom = String(this.#state.scale);
  }

  #updateZoomDisplay(): void {
    const zoomPercent = Math.round(this.#state.scale * 100);

    if (typeof window.updateZoomLevel === "function") {
      window.updateZoomLevel(zoomPercent);
    }
  }
}

// Global instance
let controller: MathWindowController | null = null;

declare global {
  interface Window {
    handleMathWindowOpen: (source: string) => void;
    mathWindowController?: MathWindowController;
    updateZoomLevel: (zoomPercent: number) => void;
  }
}

export async function initMathWindow(source: string, mathId: string): Promise<void> {
  controller = new MathWindowController();
  await controller.init(source, mathId);

  // Expose globally for Rust to call
  window.mathWindowController = controller;
}

// Function called from main markdown viewer to open window
export function openMathWindow(source: string): void {
  // Call Rust function via dioxus bridge
  window.handleMathWindowOpen(source);
}
