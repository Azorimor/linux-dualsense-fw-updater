// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

interface MdastTextNode {
  type: string;
  value?: string;
  children?: MdastTextNode[];
}

export interface MdastNode extends MdastTextNode {
  depth?: number;
  children?: MdastNode[];
}

export interface MdastFileLike {
  data?: {
    frontmatter?: {
      title?: unknown;
    };
  };
}

function extractNodeText(node: MdastNode): string {
  if (typeof node.value === 'string') {
    return node.value;
  }
  if (Array.isArray(node.children)) {
    return node.children.map(extractNodeText).join('');
  }
  return '';
}

function stripLeadingMatchingH1(children: MdastNode[], title: string): void {
  if (children.length === 0) {
    return;
  }

  const first = children[0];
  if (first.type === 'heading' && first.depth === 1) {
    const headingText = extractNodeText(first).trim().toLowerCase();
    if (headingText === title.trim().toLowerCase()) {
      children.shift();
    }
  }
}

/**
 * Remark plugin that removes a redundant leading `# <Title>` H1 heading inside
 * `<include>`d Markdown documents when it matches the page frontmatter `title`.
 * Fumadocs `<DocsPage>` already renders `<DocsTitle>` as the page's H1.
 */
export function remarkStripIncludedH1() {
  return (tree: unknown, file: unknown) => {
    const mdastTree = tree as MdastNode | null | undefined;
    const mdastFile = file as MdastFileLike | null | undefined;
    const rawTitle = mdastFile?.data?.frontmatter?.title;
    if (typeof rawTitle !== 'string' || rawTitle.trim() === '') {
      return;
    }
    if (!mdastTree || !Array.isArray(mdastTree.children)) {
      return;
    }

    for (const child of mdastTree.children) {
      if (child.type === 'root' && Array.isArray(child.children)) {
        stripLeadingMatchingH1(child.children, rawTitle);
      }
    }
  };
}
