// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

use crate::cancel::CancellationToken;

#[test]
fn token_starts_uncancelled_and_propagates_across_clones() {
    let token = CancellationToken::new();
    let clone = token.clone();
    assert!(!token.is_cancelled());
    assert!(!clone.is_cancelled());

    token.cancel();
    assert!(token.is_cancelled());
    assert!(clone.is_cancelled());

    clone.reset();
    assert!(!token.is_cancelled());
    assert!(!clone.is_cancelled());
}
