"""Tests for the upload path.

A test carries the other coverage-tag role: it verifies a rule rather than
implementing it. A suppression carries the third: it names the known-issue case
that keeps it suppressed, so the mask cannot outlive the bug silently.
"""

import pytest

from src.cleanup import retention_window_allows, upload_is_idempotent


class Artifact:
    def __init__(self, age_days):
        self.age_days = age_days


def test_cleanup_runs_after_upload():
    # VERIFIES spec-to-code:a-comment-cites-the-rule
    assert retention_window_allows(Artifact(8))
    assert not retention_window_allows(Artifact(1))


@pytest.mark.xfail(reason="KI-vendor-replays-idempotency-key", strict=True)
def test_replayed_upload_returns_the_cached_result():
    assert upload_is_idempotent()
