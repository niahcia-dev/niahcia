# AI Transport V1

Status: **CANDIDATE / pre-alpha authenticated encrypted compute transport**

## Purpose

AI Transport V1 defines the minimum transport behavior needed for wallet-to-worker compute:

- authenticated endpoint discovery;
- encrypted session establishment;
- Job submission and acceptance;
- streaming output;
- reconnect/resume;
- final ResultCommitment delivery.

It is a data-plane protocol. It does not choose chain consensus, worker eligibility, pricing, or payment authority.

## Endpoint advertisement

A signed WorkerAdvertisement SHOULD commit to an endpoint descriptor containing at least:

```text
AIEndpointDescriptorV1
- transport_protocol
- endpoint
- transport_public_key
- protocol_version
- expires_at
```

The descriptor is authenticated by the worker advertisement signature.

Wallets MUST reject expired or unsigned endpoint descriptors.

## Identity key vs traffic key

The worker's long-lived advertised transport key authenticates the endpoint.

It MUST NOT be used directly as the symmetric key for conversation traffic.

Each ComputeSession establishes fresh ephemeral session keys.

Conceptually:

```text
wallet ephemeral key
        +
worker authenticated transport key / ephemeral response
        ↓
authenticated key agreement
        ↓
fresh ComputeSession traffic keys
```

Compromise of one session traffic key should not expose unrelated sessions.

## Session handshake

The first-milestone handshake SHOULD provide:

1. worker authentication against the signed advertisement;
2. requester/session authentication using the temporary wallet-controlled session identity;
3. agreement on AI Transport protocol version;
4. binding to `ComputeSessionV1`;
5. fresh bidirectional traffic keys;
6. transcript commitment;
7. replay protection.

Candidate messages:

```text
ClientHelloV1
WorkerHelloV1
SessionConfirmV1
```

Exact cryptographic algorithms and canonical encodings require a separate interoperability lock before production.

## Channel binding

The transport session MUST bind to:

- worker_id;
- operator_id;
- ComputeSession session_id;
- requester/session identity;
- network identifier;
- protocol version.

When paid compute is used, the Job carries its PaymentAuthorization/price references separately. Transport encryption does not itself authorize spending.

## Job submission

After session establishment, the wallet sends a canonical Job request plus the encrypted input/context required for execution.

Candidate frame:

```text
JobSubmitV1
- session_id
- job_id
- job_object
- input_commitment
- encrypted_input_payload
- request_sequence
```

The worker verifies the Job before accepting execution.

## Job acceptance

The worker responds with a signed/authenticated acceptance or rejection.

Candidate acceptance:

```text
JobAcceptV1
- session_id
- job_id
- worker_id
- accepted_offer_id
- accepted_max_price
- execution_deadline
- response_sequence
```

The acceptance MUST bind the pricing terms the worker accepted.

A rejection produces no successful-job compute charge.

## Streaming output

Interactive text inference SHOULD support token/chunk streaming.

Candidate stream frame:

```text
ResultChunkV1
- session_id
- job_id
- chunk_sequence
- payload
- running_output_commitment
- final
```

Frames MUST be ordered and authenticated under the session traffic keys.

The wallet MUST NOT treat a partial stream as the final ResultCommitment.

Per-token blockchain/payment signatures are not required.

## Finalization

After inference completes, the worker sends:

```text
ResultFinalV1
- session_id
- job_id
- final_output
- ResultCommitmentV1
- metering_evidence
```

The wallet verifies the ResultCommitment and metering evidence before signing any ComputeUsageReceipt.

## Reconnect/resume

Temporary network loss should not automatically destroy a Job.

A ComputeSession SHOULD support bounded resume using an opaque resumable token or authenticated session-resume secret established during the original handshake.

Resume MUST bind:

- original session_id;
- requester identity;
- worker identity;
- latest accepted request/response sequence;
- expiry;
- fresh transport keys.

A resume handshake MUST NOT simply reuse old traffic keys.

## Stream replay

Every direction uses monotonic message/stream sequence numbers.

After reconnect, both sides identify the latest mutually accepted sequence.

Duplicate frames may be ignored, but conflicting payloads for the same authenticated sequence are protocol violations.

## Worker loss

If the worker cannot resume before the Job/session deadline:

```text
Job fails/expires
   -> no successful-job receipt
   -> wallet retains local context
   -> Worker Selection V1 chooses replacement
   -> new ComputeSession
   -> resend required context
```

The replacement worker does not inherit the failed worker's traffic keys.

## Privacy

Transport confidentiality protects data in transit.

For STANDARD private inference, the selected worker still receives plaintext context inside its execution environment after decryption.

The transport MUST NOT claim confidential compute.

Wallets SHOULD send only the minimum context required for the Job.

Workers SHOULD avoid durable plaintext logging.

## Network transport implementation

The specification defines authenticated message semantics rather than locking one socket library.

A first implementation may use a widely deployed secure transport such as TLS 1.3, QUIC, or a Noise-style authenticated channel provided the protocol-level worker/session identity binding remains explicit.

Production interoperability MUST lock one or more supported transport profiles and test vectors before independent clients are expected to interoperate.

## Denial-of-service boundary

Workers MAY require lightweight admission checks before allocating expensive model execution resources.

Transport handshake completion alone MUST NOT imply Job acceptance.

Job validation, pricing, authorization, capacity, and policy checks occur before execution.

## Invariants

1. Worker endpoint identity is authenticated from signed advertisements.
2. Every ComputeSession uses fresh traffic keys.
3. Wallet root/spending keys are not transport encryption keys.
4. Transport authentication does not authorize payment.
5. Job acceptance explicitly binds accepted price terms.
6. Streaming frames are ordered and authenticated.
7. Final payment requires a separately verified ResultCommitment/metering path.
8. Resume derives fresh traffic keys.
9. Worker failure does not require Agent-memory migration.
10. Ordinary transport exposes no conversation plaintext to the public blockchain.

## First milestone

The first implementation should prove:

1. wallet discovers signed worker endpoint;
2. wallet authenticates worker;
3. both sides establish fresh encrypted session;
4. wallet submits one SMALL TEXT_INFERENCE Job;
5. worker accepts with bound price terms;
6. worker streams output chunks;
7. worker sends ResultCommitment;
8. wallet verifies result/metering;
9. wallet signs usage receipt;
10. disconnect/reconnect resumes one in-progress or completed stream without replay ambiguity.
