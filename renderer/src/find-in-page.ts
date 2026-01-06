interface SearchState {
  query: string;
  currentIndex: number;
  highlightElements: HTMLElement[];
}

const state: SearchState = {
  query: "",
  currentIndex: 0,
  highlightElements: [],
};

type SearchCallback = (data: { count: number; current: number }) => void;

let callback: SearchCallback | null = null;

function highlightMatches(container: HTMLElement, query: string): number {
  // Clear existing highlights first
  clearHighlights();

  if (!query || query.length === 0) {
    return 0;
  }

  const textNodes: Text[] = [];
  const walker = document.createTreeWalker(container, NodeFilter.SHOW_TEXT, {
    acceptNode: (node) => {
      const parent = node.parentElement;
      // Exclude code blocks (pre), mermaid diagrams, and already highlighted text
      // Note: inline <code> tags are intentionally searchable
      if (parent?.closest("pre, .mermaid, .search-highlight")) {
        return NodeFilter.FILTER_REJECT;
      }
      return NodeFilter.FILTER_ACCEPT;
    },
  });

  let node: Node | null;
  while ((node = walker.nextNode())) {
    textNodes.push(node as Text);
  }

  const queryLower = query.toLowerCase();
  state.highlightElements = [];

  // Process each text node
  for (const textNode of textNodes) {
    const text = textNode.textContent || "";
    const textLower = text.toLowerCase();
    let startIndex = 0;

    // Find all matches in this text node
    const matches: { start: number; end: number }[] = [];
    while (true) {
      const index = textLower.indexOf(queryLower, startIndex);
      if (index === -1) break;
      matches.push({ start: index, end: index + query.length });
      startIndex = index + 1;
    }

    if (matches.length === 0) continue;

    // Replace text node with highlighted fragments
    const parent = textNode.parentNode;
    if (!parent) continue;

    const fragment = document.createDocumentFragment();
    let lastEnd = 0;

    for (const match of matches) {
      // Text before match
      if (match.start > lastEnd) {
        fragment.appendChild(document.createTextNode(text.slice(lastEnd, match.start)));
      }

      // Highlighted match
      const mark = document.createElement("mark");
      mark.className = "search-highlight";
      mark.textContent = text.slice(match.start, match.end);
      fragment.appendChild(mark);
      state.highlightElements.push(mark);

      lastEnd = match.end;
    }

    // Text after last match
    if (lastEnd < text.length) {
      fragment.appendChild(document.createTextNode(text.slice(lastEnd)));
    }

    parent.replaceChild(fragment, textNode);
  }

  return state.highlightElements.length;
}

function clearHighlights(): void {
  // Remove all highlight marks and restore original text
  for (const mark of state.highlightElements) {
    const parent = mark.parentNode;
    if (parent) {
      // Replace mark with its text content
      const textNode = document.createTextNode(mark.textContent || "");
      parent.replaceChild(textNode, mark);
      // Normalize to merge adjacent text nodes
      parent.normalize();
    }
  }
  state.highlightElements = [];
  state.currentIndex = 0;
}

function navigateToMatch(direction: "next" | "prev"): number {
  if (state.highlightElements.length === 0) return 0;

  // Remove active class from current
  const current = state.highlightElements[state.currentIndex];
  current?.classList.remove("search-highlight-active");

  // Calculate new index
  if (direction === "next") {
    state.currentIndex = (state.currentIndex + 1) % state.highlightElements.length;
  } else {
    state.currentIndex =
      (state.currentIndex - 1 + state.highlightElements.length) % state.highlightElements.length;
  }

  // Add active class to new current and scroll into view
  const next = state.highlightElements[state.currentIndex];
  next?.classList.add("search-highlight-active");
  next?.scrollIntoView({ behavior: "smooth", block: "center" });

  return state.currentIndex + 1; // 1-based for display
}

export function find(query: string): void {
  state.query = query;
  const container = document.querySelector(".markdown-body");
  if (!container) {
    callback?.({ count: 0, current: 0 });
    return;
  }

  const count = highlightMatches(container as HTMLElement, query);
  state.currentIndex = count > 0 ? 0 : -1;

  // Activate first match (no auto-scroll to avoid focus issues with IME)
  if (count > 0) {
    state.highlightElements[0]?.classList.add("search-highlight-active");
  }

  callback?.({ count, current: count > 0 ? 1 : 0 });
}

export function navigate(direction: "next" | "prev"): void {
  const current = navigateToMatch(direction);
  callback?.({ count: state.highlightElements.length, current });
}

export function clear(): void {
  clearHighlights();
  callback?.({ count: 0, current: 0 });
}

export function setup(cb: SearchCallback): void {
  callback = cb;
}
