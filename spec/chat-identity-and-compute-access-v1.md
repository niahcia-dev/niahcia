# Niahcia Wallet Chat, Identity, Privacy, and Compute Access Protocol v1

Status: Draft
Version: 1

## Purpose

Niahcia Chat is a wallet capability. Websites such as niahcia.com are portals to that capability, not owners of a user's conversations, identity, keys, or AI relationship. Desktop wallets, mobile wallets, and independent clients SHOULD be able to use the same protocol.

## Core principles

1. A wallet-controlled Niahcia identity MAY own portable AI chat sessions.
2. niahcia.com MUST NOT be required for protocol-level chat identity or session ownership.
3. Basic chat MAY be provided without payment under implementation-defined allowances.
4. Advanced AI work MAY require explicit, bounded usage authorization and payment.
5. Persistent private chat content MUST be encrypted by default.
6. Conversation plaintext MUST NOT be committed to the public blockchain by this protocol.
7. Wallet spending keys MUST NOT be used directly as chat-content encryption keys.
8. Pricing, model names, free-tier limits, storage providers, and user interfaces MUST NOT be hard-coded into consensus.

## Wallet identity and portal access

A wallet MAY authenticate to a portal or client by signing a domain-separated challenge binding the protocol/version domain, requesting origin, Niahcia network identifier, nonce, issued-at time, expiration time, and requested capabilities.

Authentication proves control of the relevant identity key. It MUST NOT imply unrestricted wallet access, chat decryption authority, or spending authority. A portal SHOULD receive only the minimum delegated authority necessary for the requested session.

## Key separation

Wallet implementations SHOULD maintain separate cryptographic authority domains for value/spending, identity/authentication, chat encryption, devices/temporary sessions, and agents/delegated capabilities. Compromise or delegation of one authority SHOULD NOT automatically grant the others.

A wallet's spending private key MUST NOT be used directly to encrypt conversation content.

## ChatSessionV1

A portable conversation MAY be represented by a `ChatSessionV1` descriptor:

```text
ChatSessionV1
  version
  session_id
  owner_identity
  created_at
  capability_set
  conversation_root
  storage_descriptor
  encryption_descriptor
  metadata_commitment
```

The descriptor contains only information required for identity, discovery, authorization, synchronization, and integrity. Conversation plaintext is not part of it. A session MAY remain entirely local and need not be registered on-chain.

## Encrypted conversation state

Persistent private chat content MUST be encrypted by default, including prompts, responses, conversation titles, attachments, persistent memories, agent state, private tool results, and private conversation indexes and metadata where practical.

Each conversation SHOULD use an independent random content-encryption key rather than one permanent wallet-wide chat key. Conversation keys MAY be wrapped for authorized devices or identities.

The design SHOULD support key rotation, device authorization/revocation, selective sharing, export, migration, and recovery without exposing spending keys. Remote storage providers SHOULD receive ciphertext and the minimum metadata necessary for storage/synchronization. No consensus rule SHALL require a single conversation storage provider.

## On-chain privacy boundary

The chain MAY contain commitments, hashes, authorization records, payment/settlement data, verification data, and minimal required protocol metadata.

The protocol MUST NOT require prompts, responses, attachments, memories, or conversation plaintext to be published on-chain.

## Inference privacy boundary

Encryption at rest and in transit does not by itself permit conventional LLM inference over ciphertext. For ordinary inference, an authorized execution environment may need temporary access to the plaintext context required for the job.

Implementations MUST minimize this exposure and SHOULD disclose only necessary context, use authenticated encrypted transport and short-lived job/session keys, avoid persistent plaintext logging, erase temporary plaintext and keys when no longer required, and re-encrypt results before persistent storage.

The protocol MUST NOT claim that a standard compute worker cannot observe plaintext merely because stored conversations are encrypted.

## Privacy execution profiles

ExecutionProfile or a successor primitive SHOULD advertise privacy properties. Profiles may include standard private execution (encrypted transport/storage with authorized plaintext inference), confidential execution using attested/confidential computing where supported, and future cryptographic execution such as MPC/FHE if practical. These technologies are not consensus requirements in v1.

## Compute access and payment

Chat requests requiring network compute SHOULD use Niahcia's general job/execution primitives rather than a separate chat compute market. Free/basic access is an entitlement policy, not a distinct consensus class of AI.

Paid execution SHOULD follow:

```text
request
  -> quote/estimate
  -> explicit maximum-spend authorization
  -> job dispatch
  -> execution
  -> verification/result commitment
  -> settlement of actual authorized usage
  -> release of unused authorization
```

Maximum-spend authorization MUST NOT permit spending beyond the authorized amount. Settlement SHOULD compose with Niahcia Job, ExecutionProfile, ComputeWorker, Capability, PaymentPlan, VerificationPolicy, and ResultCommitment primitives.

## Wallet-native operation

A conforming wallet MAY authenticate an identity, create/resume a session, decrypt authorized conversation state, discover AI capabilities, submit basic or paid jobs, display estimates, authorize bounded payment, retrieve and verify results, encrypt responses, and synchronize encrypted conversation state.

A web portal or third-party client MAY expose the same capabilities through explicitly delegated wallet authority. The user's AI relationship MUST NOT depend on continued availability of niahcia.com.

## Separation of authority

Identity authentication, conversation decryption, capability authorization, payment authorization, device/session authorization, and agent delegation are distinct authorities and SHOULD remain separable.

## Non-goals for v1

This specification intentionally does not fix NIAH-denominated chat prices, free-tier quotas, specific models, context-window sizes, a mandatory storage network, wallet UI, memory implementation, confidential-computing hardware, or frontend.

## Forward compatibility

Future versions may define canonical encodings, signed authentication challenges, encrypted conversation manifests, delegated session keys, device synchronization, spending permits, recovery mechanisms, agent-owned sessions, confidential-compute attestations, and privacy-preserving inference profiles.

Those additions MUST preserve the central v1 properties: the wallet controls the AI identity and chat keys; portals are replaceable; private persistent chat is encrypted by default; and conversation plaintext is not public chain data.
