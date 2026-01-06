---
title: Frontmatter Test
author: Arto Team
date: 2025-01-07
draft: false
version: 1.0
tags:
  - markdown
  - frontmatter
  - test
settings:
  theme: dark
  sidebar: true
empty_value:
---

# Frontmatter Test

This file is for testing the frontmatter table display.

## What Should Be Displayed

A collapsible table should appear at the top with the following type styling:

- **String**: Normal text (title, author, date)
- **Boolean**: Highlighted in blue (draft, settings.sidebar)
- **Number**: Highlighted in green (version)
- **List**: Displayed as bullet points (tags)
- **Object**: Nested table (settings)
- **null**: Italic "null" (empty_value)

## Normal Content

This is normal Markdown content displayed below the frontmatter.

> [!NOTE]
> Frontmatter is YAML-formatted metadata enclosed by `---`.
> It must be placed at the beginning of the file.

```rust
fn main() {
    println!("Hello, Arto!");
}
```
