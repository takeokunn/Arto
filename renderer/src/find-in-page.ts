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

/**
 * Match information for displaying in the Search tab.
 */
export interface SearchMatch {
  /** 0-based index of this match */
  index: number;
  /** The matched text itself */
  text: string;
  /** Surrounding context including the match */
  context: string;
  /** Start position of match within context */
  contextStart: number;
  /** End position of match within context */
  contextEnd: number;
}

/**
 * Full search result including all match details.
 */
export interface SearchResult {
  query: string;
  total: number;
  current: number;
  matches: SearchMatch[];
}

type SearchCallback = (data: {
  count: number;
  current: number;
  query: string;
  matches: SearchMatch[];
}) => void;

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
    callback?.({ count: 0, current: 0, query: "", matches: [] });
    return;
  }

  const count = highlightMatches(container as HTMLElement, query);
  state.currentIndex = count > 0 ? 0 : -1;

  // Activate first match (no auto-scroll to avoid focus issues with IME)
  if (count > 0) {
    state.highlightElements[0]?.classList.add("search-highlight-active");
  }

  const matches = collectMatches();
  callback?.({ count, current: count > 0 ? 1 : 0, query: state.query, matches });
}

export function navigate(direction: "next" | "prev"): void {
  const current = navigateToMatch(direction);
  const matches = collectMatches();
  callback?.({ count: state.highlightElements.length, current, query: state.query, matches });
}

export function clear(): void {
  state.query = "";
  clearHighlights();
  callback?.({ count: 0, current: 0, query: "", matches: [] });
}

export function setup(cb: SearchCallback): void {
  callback = cb;
}

/**
 * Re-apply the current search query after DOM changes (e.g., tab switch).
 * This preserves the search highlight across tab navigation.
 */
export function reapply(): void {
  if (!state.query) {
    return;
  }
  // Re-run search with stored query
  find(state.query);
}

/**
 * Navigate directly to a specific match by index.
 * Used by the Search tab for clicking on match items.
 */
export function navigateTo(index: number): void {
  if (index < 0 || index >= state.highlightElements.length) {
    return;
  }

  // Remove active class from current match
  const current = state.highlightElements[state.currentIndex];
  current?.classList.remove("search-highlight-active");

  // Update index and activate new match
  state.currentIndex = index;
  const target = state.highlightElements[index];
  target?.classList.add("search-highlight-active");
  target?.scrollIntoView({ behavior: "smooth", block: "center" });

  // Notify callback with unified format
  const newCurrent = index + 1;
  const matches = collectMatches();
  callback?.({
    count: state.highlightElements.length,
    current: newCurrent,
    query: state.query,
    matches,
  });
}

/**
 * Collect context around an element (text before and after).
 */
function getContext(
  element: HTMLElement,
  maxChars: number,
): { text: string; matchStart: number; matchEnd: number } {
  const matchText = element.textContent || "";

  // Get text from siblings and parent text nodes
  const before = getTextBefore(element, maxChars);
  const after = getTextAfter(element, maxChars);

  const text = before + matchText + after;
  const matchStart = before.length;
  const matchEnd = matchStart + matchText.length;

  return { text, matchStart, matchEnd };
}

/**
 * Get text content before an element, up to maxChars.
 */
function getTextBefore(element: HTMLElement, maxChars: number): string {
  let text = "";
  let node: Node | null = element;

  // Walk backwards through siblings and parent's previous siblings
  while (node && text.length < maxChars) {
    if (node.previousSibling) {
      node = node.previousSibling;
      const content = getNodeTextContent(node);
      text = content + text;
    } else {
      // Move up to parent and continue
      node = node.parentElement;
      if (node && node.closest(".markdown-body")) {
        continue;
      }
      break;
    }
  }

  // Trim to maxChars from the end
  if (text.length > maxChars) {
    text = "..." + text.slice(-maxChars);
  }

  return text;
}

/**
 * Get text content after an element, up to maxChars.
 */
function getTextAfter(element: HTMLElement, maxChars: number): string {
  let text = "";
  let node: Node | null = element;

  // Walk forwards through siblings and parent's next siblings
  while (node && text.length < maxChars) {
    if (node.nextSibling) {
      node = node.nextSibling;
      const content = getNodeTextContent(node);
      text = text + content;
    } else {
      // Move up to parent and continue
      node = node.parentElement;
      if (node && node.closest(".markdown-body")) {
        continue;
      }
      break;
    }
  }

  // Trim to maxChars from the start
  if (text.length > maxChars) {
    text = text.slice(0, maxChars) + "...";
  }

  return text;
}

/**
 * Get text content of a node, handling different node types.
 */
function getNodeTextContent(node: Node): string {
  if (node.nodeType === Node.TEXT_NODE) {
    return node.textContent || "";
  }
  if (node.nodeType === Node.ELEMENT_NODE) {
    const el = node as HTMLElement;
    // Skip search highlight marks to get actual text
    if (el.classList.contains("search-highlight")) {
      return el.textContent || "";
    }
    return el.textContent || "";
  }
  return "";
}

/**
 * Collect all match information for the Search tab.
 */
function collectMatches(): SearchMatch[] {
  const matches: SearchMatch[] = [];
  const contextChars = 30;

  for (let i = 0; i < state.highlightElements.length; i++) {
    const el = state.highlightElements[i];
    const text = el.textContent || "";
    const context = getContext(el, contextChars);

    matches.push({
      index: i,
      text,
      context: context.text,
      contextStart: context.matchStart,
      contextEnd: context.matchEnd,
    });
  }

  return matches;
}
