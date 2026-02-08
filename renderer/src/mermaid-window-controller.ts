import mermaid from "mermaid";
import type { Theme } from "./theme";
import { buildMermaidThemeConfig } from "./mermaid-theme";
import { fixTextContrast } from "./mermaid-contrast";
import {
  createBlobPromise,
  createCanvasFromSvg,
  convertSvgToDataUrl,
  findSvgElement,
  getSvgDimensions,
} from "./code-copy";
import { BaseViewerController } from "./base-viewer-controller.ts";

class MermaidWindowController extends BaseViewerController {
  async init(source: string, diagramId: string): Promise<void> {
    this.container = document.getElementById("mermaid-window-canvas");
    this.wrapper = document.getElementById("mermaid-diagram-wrapper");
    this.contentContainer = document.getElementById("mermaid-diagram-container");

    if (!this.container || !this.wrapper || !this.contentContainer) {
      throw new Error("Viewer container not found");
    }

    // Initialize mermaid with current theme
    const currentTheme = document.body.getAttribute("data-theme") as Theme;
    this.#initializeMermaidTheme(currentTheme || "light");

    // Render the Mermaid diagram
    await this.#renderDiagram(source, diagramId);

    // Setup event listeners
    this.setupBaseEventListeners();

    // Listen for theme changes
    document.addEventListener("arto:theme-changed", ((event: CustomEvent) => {
      this.setTheme(event.detail);
    }) as EventListener);

    // Initial fit to window
    setTimeout(() => this.fitToWindow(), 100);
  }

  setTheme(theme: string): void {
    // Update body theme attribute
    document.body.setAttribute("data-theme", theme);

    // Re-initialize mermaid with new theme
    this.#initializeMermaidTheme(theme as Theme);

    // Re-render the diagram with new theme asynchronously
    if (this.contentContainer) {
      const source = this.contentContainer.getAttribute("data-mermaid-source");
      const diagramId = this.contentContainer.getAttribute("data-diagram-id");

      if (source && diagramId) {
        // Re-render asynchronously without blocking
        this.#renderDiagram(source, diagramId)
          .then(() => {
            // Restore zoom and position after re-render
            this.updateTransform();
          })
          .catch((error) => {
            console.error("Failed to re-render diagram:", error);
          });
      }
    }

    console.log("Theme changed to:", theme);
  }

  /**
   * Copy the current diagram as a PNG image to clipboard
   * @returns true if successful, false otherwise
   */
  async copyAsImage(): Promise<boolean> {
    if (!navigator.clipboard?.write) {
      console.error("Clipboard API not available");
      return false;
    }

    if (!this.contentContainer) {
      console.error("Diagram container not found");
      return false;
    }

    try {
      const svg = findSvgElement(this.contentContainer);
      const dimensions = getSvgDimensions(svg);
      const canvas = createCanvasFromSvg(svg, dimensions);
      const svgDataUrl = convertSvgToDataUrl(svg, dimensions);

      // Create blob promise synchronously to preserve user gesture context
      const blobPromise = createBlobPromise(canvas, svgDataUrl);

      // Write to clipboard with promise (WebKit-compatible approach)
      await navigator.clipboard.write([new ClipboardItem({ "image/png": blobPromise })]);

      return true;
    } catch (error) {
      console.error("Failed to copy diagram as image:", error);
      return false;
    }
  }

  protected getContentDimensions(): { width: number; height: number } | null {
    if (!this.contentContainer) return null;

    const svg = this.contentContainer.querySelector("svg");
    if (!svg) return null;

    return this.#getViewerDimensions(svg);
  }

  #initializeMermaidTheme(theme: Theme): void {
    const config = buildMermaidThemeConfig(theme);
    mermaid.initialize({
      startOnLoad: false,
      ...config,
      securityLevel: "loose",
      fontFamily: "inherit",
    });
  }

  async #renderDiagram(source: string, diagramId: string): Promise<void> {
    try {
      const { svg } = await mermaid.render(`viewer-${diagramId}`, source);
      if (this.contentContainer) {
        this.contentContainer.innerHTML = svg;

        // Set explicit pixel dimensions for CSS zoom to work properly.
        // Use viewBox dimensions rather than getBBox() because Mermaid's
        // Gantt renderer sets viewBox to the full intended area ("0 0 w h")
        // while getBBox() may return smaller content bounds, creating a
        // mismatch between viewBox and pixel dimensions that causes the
        // diagram to appear extremely small after zoom scaling.
        const svgElement = this.contentContainer.querySelector("svg");
        if (svgElement) {
          // Fix text contrast for nodes with custom fill colors
          fixTextContrast(svgElement as SVGSVGElement);

          const dims = this.#getViewerDimensions(svgElement);
          svgElement.setAttribute("width", String(dims.width));
          svgElement.setAttribute("height", String(dims.height));
          // Remove responsive max-width that conflicts with explicit dimensions
          svgElement.style.removeProperty("max-width");
        }

        // Store source and ID for theme switching
        this.contentContainer.setAttribute("data-mermaid-source", source);
        this.contentContainer.setAttribute("data-diagram-id", diagramId);
      }
    } catch (error) {
      console.error("Failed to render diagram:", error);
      if (this.contentContainer) {
        this.contentContainer.innerHTML = `
          <div style="color: red; padding: 2rem;">
            <strong>Rendering Error:</strong><br/>
            <pre style="white-space: pre-wrap;">${error}</pre>
          </div>
        `;
      }
    }
  }

  // Get the intended diagram dimensions from the SVG's viewBox attribute.
  // Prefer viewBox over getBBox() because Mermaid sets viewBox to the full
  // intended rendering area during diagram generation, whereas getBBox()
  // returns only the tight content bounds which may be smaller (especially
  // for Gantt charts where viewBox="0 0 w h" encompasses padding/axis area).
  #getViewerDimensions(svg: SVGSVGElement): { width: number; height: number } {
    const viewBox = svg.getAttribute("viewBox");
    if (viewBox) {
      const parts = viewBox.split(/[\s,]+/).map(Number);
      if (parts.length === 4 && parts[2] > 0 && parts[3] > 0) {
        return { width: parts[2], height: parts[3] };
      }
    }
    // Fall back to getBBox for SVGs without viewBox
    const bbox = svg.getBBox();
    return { width: bbox.width || 1, height: bbox.height || 1 };
  }
}

// Global instance
let controller: MermaidWindowController | null = null;

declare global {
  interface Window {
    handleMermaidWindowOpen: (source: string) => void;
    mermaidWindowController?: MermaidWindowController;
  }
}

export async function initMermaidWindow(source: string, diagramId: string): Promise<void> {
  controller = new MermaidWindowController();
  await controller.init(source, diagramId);

  // Expose globally for Rust to call
  window.mermaidWindowController = controller;
}

// Function called from main markdown viewer to open window
export function openMermaidWindow(source: string): void {
  // Call Rust function via dioxus bridge
  window.handleMermaidWindowOpen(source);
}
