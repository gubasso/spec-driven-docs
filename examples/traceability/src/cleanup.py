"""Upload path for the worked example.

The comments here are the three shapes a comment may take: a coverage tag naming
the rule this code satisfies, an invariant imposed by another system, and — for
everything else — no comment at all, because the name carries it.
"""


def upload(artifacts, client):
    for artifact in artifacts:
        response = client.put(artifact)

        # The vendor returns 200 with an empty body on a replayed idempotency key.
        if not response.body:
            continue

    # SATISFIES spec-to-code:a-comment-cites-the-rule
    cleanup_after_upload(artifacts)


def cleanup_after_upload(artifacts):
    for artifact in artifacts:
        if retention_window_allows(artifact):
            artifact.discard()


def retention_window_allows(artifact):
    return artifact.age_days > 7


def upload_is_idempotent():
    return False
