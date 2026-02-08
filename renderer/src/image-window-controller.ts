import { BaseViewerController } from "./base-viewer-controller";

class ImageWindowController extends BaseViewerController {
  #imgElement: HTMLImageElement | null = null;

  async init(src: string): Promise<void> {
    this.container = document.getElementById("image-window-canvas");
    this.wrapper = document.getElementById("image-wrapper");
    this.contentContainer = document.getElementById("image-container");

    if (!this.container || !this.wrapper || !this.contentContainer) {
      throw new Error("Image viewer container not found");
    }

    // Create and load the image from the src passed by Rust
    await this.#loadImage(src);

    // Setup event listeners
    this.setupBaseEventListeners();

    // Listen for theme changes (only affects window chrome, not image content)
    document.addEventListener("arto:theme-changed", ((event: CustomEvent) => {
      document.body.setAttribute("data-theme", event.detail);
    }) as EventListener);

    // Initial fit to window after image loads
    setTimeout(() => this.fitToWindow(), 100);
  }

  protected getContentDimensions(): { width: number; height: number } | null {
    if (!this.#imgElement) return null;
    return {
      width: this.#imgElement.naturalWidth,
      height: this.#imgElement.naturalHeight,
    };
  }

  async #loadImage(src: string): Promise<void> {
    const img = new Image();
    img.draggable = false;

    await new Promise<void>((resolve, reject) => {
      img.onload = () => resolve();
      img.onerror = () => {
        if (this.contentContainer) {
          this.contentContainer.innerHTML = `
            <div style="color: var(--text-secondary); padding: 2rem; text-align: center;">
              <strong>Failed to load image</strong>
            </div>
          `;
        }
        reject(new Error("Failed to load image"));
      };
      img.src = src;
    });

    if (img.naturalWidth === 0) {
      if (this.contentContainer) {
        this.contentContainer.innerHTML = `
          <div style="color: red; padding: 2rem;">
            <strong>Image Load Error:</strong><br/>
            <pre style="white-space: pre-wrap;">Failed to load image</pre>
          </div>
        `;
      }
      throw new Error("Failed to load image");
    }

    img.style.display = "block";
    img.style.width = `${img.naturalWidth}px`;
    img.style.height = `${img.naturalHeight}px`;

    if (this.contentContainer) {
      this.contentContainer.appendChild(img);
    }

    this.#imgElement = img;
  }
}

export async function initImageWindow(src: string): Promise<void> {
  const controller = new ImageWindowController();
  await controller.init(src);

  // Free the embedded data URL from memory (no longer needed after image loads)
  delete (window as Record<string, unknown>)._imageDataUrl;
}
