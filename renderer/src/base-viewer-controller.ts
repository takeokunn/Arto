export interface ViewerState {
  scale: number;
  offsetX: number;
  offsetY: number;
  isDragging: boolean;
  lastMouseX: number;
  lastMouseY: number;
}

export abstract class BaseViewerController {
  protected container: HTMLElement | null = null;
  protected wrapper: HTMLElement | null = null;
  protected contentContainer: HTMLElement | null = null;
  protected maxZoom: number = 100.0;
  protected state: ViewerState = {
    scale: 1.0,
    offsetX: 0,
    offsetY: 0,
    isDragging: false,
    lastMouseX: 0,
    lastMouseY: 0,
  };

  /** Subclasses must implement to return the natural content dimensions */
  protected abstract getContentDimensions(): { width: number; height: number } | null;

  protected setupBaseEventListeners(): void {
    if (!this.container) return;

    // Disable native WKWebView context menu
    document.addEventListener("contextmenu", (e) => e.preventDefault());

    // Keyboard shortcuts
    document.addEventListener("keydown", this.handleKeyDown.bind(this));

    // Mouse events for dragging
    this.container.addEventListener("mousedown", this.handleMouseDown.bind(this));
    document.addEventListener("mousemove", this.handleMouseMove.bind(this));
    document.addEventListener("mouseup", this.handleMouseUp.bind(this));

    // Scroll events
    this.container.addEventListener("wheel", this.handleWheel.bind(this), { passive: false });

    // Double-click to fit
    this.container.addEventListener("dblclick", this.handleDoubleClick.bind(this));
  }

  protected handleKeyDown(event: KeyboardEvent): void {
    const isCmdOrCtrl = event.metaKey || event.ctrlKey;

    if (isCmdOrCtrl) {
      if (event.key === "=" || event.key === "+") {
        event.preventDefault();
        this.zoom(0.1);
      } else if (event.key === "-") {
        event.preventDefault();
        this.zoom(-0.1);
      } else if (event.key === "0") {
        event.preventDefault();
        this.fitToWindow();
      }
    }
  }

  protected handleMouseDown(event: MouseEvent): void {
    if (event.button === 0) {
      this.state.isDragging = true;
      this.state.lastMouseX = event.clientX;
      this.state.lastMouseY = event.clientY;
      if (this.container) {
        this.container.style.cursor = "grabbing";
      }
    }
  }

  protected handleMouseMove(event: MouseEvent): void {
    if (this.state.isDragging) {
      const dx = event.clientX - this.state.lastMouseX;
      const dy = event.clientY - this.state.lastMouseY;

      this.state.offsetX += dx;
      this.state.offsetY += dy;

      this.state.lastMouseX = event.clientX;
      this.state.lastMouseY = event.clientY;

      this.updateTransform();
    }
  }

  protected handleMouseUp(): void {
    this.state.isDragging = false;
    if (this.container) {
      this.container.style.cursor = "grab";
    }
  }

  protected handleWheel(event: WheelEvent): void {
    event.preventDefault();

    const deltaScale = this.getDeltaModeScale(event.deltaMode);
    const deltaY = event.deltaY * deltaScale;
    const ZOOM_SCALE = 0.01;
    const zoomFactor = Math.exp(-deltaY * ZOOM_SCALE);

    const oldScale = this.state.scale;
    const newScale = Math.max(0.1, Math.min(this.maxZoom, oldScale * zoomFactor));

    if (newScale !== oldScale) {
      const rect = this.container!.getBoundingClientRect();
      const mouseX = event.clientX - rect.left;
      const mouseY = event.clientY - rect.top;

      const contentX = (mouseX - this.state.offsetX) / oldScale;
      const contentY = (mouseY - this.state.offsetY) / oldScale;

      this.state.offsetX = mouseX - contentX * newScale;
      this.state.offsetY = mouseY - contentY * newScale;
      this.state.scale = newScale;

      this.updateTransform();
      this.updateZoomDisplay();
    }
  }

  protected handleDoubleClick(): void {
    this.fitToWindow();
  }

  protected getDeltaModeScale(deltaMode: number): number {
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

  protected zoom(delta: number): void {
    const newScale = Math.max(0.1, Math.min(this.maxZoom, this.state.scale + delta));

    if (this.container) {
      const centerX = this.container.clientWidth / 2;
      const centerY = this.container.clientHeight / 2;

      const oldScale = this.state.scale;
      const scaleRatio = newScale / oldScale;

      this.state.offsetX = centerX - (centerX - this.state.offsetX) * scaleRatio;
      this.state.offsetY = centerY - (centerY - this.state.offsetY) * scaleRatio;
    }

    this.state.scale = newScale;
    this.updateTransform();
    this.updateZoomDisplay();
  }

  protected fitToWindow(): void {
    const dims = this.getContentDimensions();
    if (!this.container || !dims) return;

    const { width, height } = dims;
    if (width === 0 || height === 0) return;

    const padding = 40;
    const availableWidth = this.container.clientWidth - padding * 2;
    const availableHeight = this.container.clientHeight - padding * 2;

    const scaleX = availableWidth / width;
    const scaleY = availableHeight / height;
    const scale = Math.min(scaleX, scaleY, this.maxZoom);

    this.state.scale = scale;

    const scaledWidth = width * scale;
    const scaledHeight = height * scale;

    this.state.offsetX = (this.container.clientWidth - scaledWidth) / 2;
    this.state.offsetY = (this.container.clientHeight - scaledHeight) / 2;

    this.updateTransform();
    this.updateZoomDisplay();
  }

  protected updateTransform(animate = false): void {
    if (!this.wrapper || !this.contentContainer) return;

    if (animate) {
      this.wrapper.style.transition = "transform 0.3s ease-out";
      this.contentContainer.style.transition = "zoom 0.3s ease-out";
    } else {
      this.wrapper.style.transition = "none";
      this.contentContainer.style.transition = "none";
    }

    this.wrapper.style.transform = `translate(${this.state.offsetX}px, ${this.state.offsetY}px)`;
    this.contentContainer.style.zoom = String(this.state.scale);
  }

  protected updateZoomDisplay(): void {
    const zoomPercent = Math.round(this.state.scale * 100);
    if (typeof window.updateZoomLevel === "function") {
      window.updateZoomLevel(zoomPercent);
    }
  }
}

declare global {
  interface Window {
    updateZoomLevel: (zoomPercent: number) => void;
  }
}
