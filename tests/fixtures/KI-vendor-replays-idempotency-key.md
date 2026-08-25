---
upstream: https://github.com/example/vendor-sdk/issues/1234
affects: upload client
state: masked
workaround: treat an empty 200 body as the cached result rather than a failed write
retire_when: vendor-sdk release >= 2.4.0
---

# The vendor returns 200 with an empty body on a replayed idempotency key

## Symptom

A retried upload receives HTTP 200 with a zero-length body. The client cannot tell the replay apart from a successful write that returned nothing, so a naive parse raises on the empty document.

## How it works

Three parts move: the client's retry timer, the vendor's idempotency cache, and the response writer that serves a cache hit.

1. The client `PUT`s an artifact with `Idempotency-Key: 7f3a`. The vendor stores the artifact and answers `200` with the created document, `Content-Length: 412`.
2. The acknowledgement is lost in transit and the client's timer fires. It re-sends the identical request, same key.
3. The vendor recognises the key, skips the write, and takes the cache-hit path. That path sets the status from the cached entry but serves no body: the response is `200` with `Content-Length: 0`.
4. The client parses the body as the created document and raises on end-of-input.

The cause is in step 3: the cache-hit path reuses the status code of the original response without reusing its body. The first reading a triager reaches for is that the retry raced a delete, which is wrong — the access log shows no delete, and the artifact is still readable afterwards. The smallest fix upstream is for the cache-hit path to serve the stored document rather than an empty body.

## Signal

| Signal                                   | Expected result                                  |
| ---------------------------------------- | ------------------------------------------------ |
| `rg 'PUT /artifacts' access.log \| tail` | two entries sharing one `Idempotency-Key` header |
| response `Content-Length`                | `0` on the second entry, non-zero on the first   |

## Workaround

`src/cleanup.py` treats an empty body as the cached result. The comment beside it states the vendor invariant, because no rule of this project was agreed to produce that branch.

`tests/test_cleanup.py` carries a strict expected failure naming this case, so the suite turns red the moment the vendor ships the fix and the mask can be removed.

## Report

```text
PUT with a replayed Idempotency-Key returns 200 with an empty body

Re-sending an identical PUT with the same Idempotency-Key returns 200 and
Content-Length: 0, while the first request returned 200 with the created
document. The cache-hit path appears to reuse the status of the original
response without reusing its body.

Expected: a replayed request returns the stored document.
Observed: a replayed request returns an empty body.
Affects: vendor-sdk 2.3.1, upload client.
```
