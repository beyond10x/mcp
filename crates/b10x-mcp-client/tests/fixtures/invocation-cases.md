# Connectors invocation corpus

`invocation-cases.json` is copied byte-for-byte from
`beyond10x/connectors` commit `b360aaaa771ad9a31cab46ff4249015b74a4d50f`,
`adapters/mcp/contracts/client/v1alpha1/invocation-cases.json`.
SHA-256: `5a0d73a4d1628812b3019c40049c9956afb74524e77cec99a241d39eb001e13a`.

The runtime replay exercises the thirty complete-response semantic vectors over
actual HTTP after actual setup and discovery. Only the envelope request id changes
from 1 to 3 (same byte length) to account for those preceding exchanges. The fixture
does not parse/reserialize the supplied response. The test compares preserved
result/error fields and raw response bytes with the literal corpus.

Ten document-only input/framing vectors are explicitly excluded from this replay:
two caller-validity flags have no invalid wire input; two request-limit vectors
cannot apply their one-byte limit to connection setup; and six prefix/loss/bound
vectors require transport controls instead of a complete-response fixture. They
remain in the unchanged corpus, and are not reported as runtime passes here.
Native strict-HTTP tests and invocation incomplete/deadline cases independently
exercise their applicable transport behavior. This replay does not prove consumer
error mapping, authority, or Connectors runtime integration.
