// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

export const SITE_NAME = 'Unofficial Linux DualSense Firmware Updater';
export const MIN_DESCRIPTION_LENGTH = 40;
export const RECOMMENDED_MAX_DESCRIPTION_LENGTH = 160;
export const HARD_MAX_DESCRIPTION_LENGTH = 200;

export interface BreadcrumbItem {
  readonly name: string;
  readonly url: string;
}

export interface TechArticleInput {
  readonly title: string;
  readonly description: string;
  readonly url: string;
  readonly imageUrl?: string;
}

export interface MetaDescriptionValidation {
  readonly valid: boolean;
  readonly length: number;
  readonly reason?: string;
}

/**
 * Serialize JSON-LD data for embedding inside `<script type="application/ld+json">`,
 * escaping `<` as `\u003c` to prevent closing-tag injection.
 */
export function serializeJsonLd(data: unknown): string {
  const json = JSON.stringify(data) ?? 'null';
  return json.replace(/</g, '\\u003c');
}

/**
 * Build a Schema.org `BreadcrumbList` JSON-LD object.
 */
export function buildBreadcrumbJsonLd(items: ReadonlyArray<BreadcrumbItem>) {
  return {
    '@context': 'https://schema.org',
    '@type': 'BreadcrumbList',
    itemListElement: items.map((item, index) => ({
      '@type': 'ListItem',
      position: index + 1,
      name: item.name,
      item: item.url,
    })),
  };
}

/**
 * Build a Schema.org `TechArticle` JSON-LD object for documentation pages.
 */
export function buildTechArticleJsonLd({
  title,
  description,
  url,
  imageUrl,
}: TechArticleInput) {
  return {
    '@context': 'https://schema.org',
    '@type': 'TechArticle',
    headline: title,
    description,
    url,
    mainEntityOfPage: {
      '@type': 'WebPage',
      '@id': url,
    },
    ...(imageUrl ? { image: imageUrl } : {}),
    author: {
      '@type': 'Person',
      name: 'Azorimor',
      url: 'https://github.com/Azorimor',
    },
    publisher: {
      '@type': 'Organization',
      name: SITE_NAME,
    },
    inLanguage: 'en',
  };
}

/**
 * Combine multiple Schema.org nodes into a single `@graph` JSON-LD document.
 */
export function buildJsonLdGraph(nodes: ReadonlyArray<object>) {
  return {
    '@context': 'https://schema.org',
    '@graph': nodes.map((node) =>
      Object.fromEntries(
        Object.entries(node).filter(([key]) => key !== '@context'),
      ),
    ),
  };
}

/**
 * Validate a meta description string against minimum (40) and hard maximum (200)
 * character bounds.
 */
export function validateMetaDescription(
  description: string,
): MetaDescriptionValidation {
  const length = description.trim().length;
  if (length < MIN_DESCRIPTION_LENGTH) {
    return {
      valid: false,
      length,
      reason: `Description is too short (${length} chars; minimum ${MIN_DESCRIPTION_LENGTH}).`,
    };
  }
  if (length > HARD_MAX_DESCRIPTION_LENGTH) {
    return {
      valid: false,
      length,
      reason: `Description is too long (${length} chars; maximum ${HARD_MAX_DESCRIPTION_LENGTH}).`,
    };
  }
  return { valid: true, length };
}
