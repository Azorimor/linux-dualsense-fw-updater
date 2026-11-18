// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

import { docsLlms } from '@/lib/source';

export const dynamic = 'force-static';
export const revalidate = false;

export async function GET() {
  return (
    (await docsLlms.serve('/llms.txt')) ??
    new Response(null, { status: 404 })
  );
}
