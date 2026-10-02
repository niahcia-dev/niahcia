# PrimaryAgentKnowledgeV1

Status: **CANDIDATE / pre-alpha knowledge and learning architecture**

## Purpose

`PrimaryAgentKnowledgeV1` defines how NIAHCIA's Primary Agent may accumulate durable knowledge from users, Agents, tools, execution results, public sources, and protocol evidence without treating arbitrary generated content as truth or absorbing private network memory.

This specification deliberately separates **knowledge accumulation** from **model training**. The Primary Agent can improve its durable knowledge and routing behavior without continuously rewriting model weights.

## Central rules

> Knowledge is an evidence-bearing claim, not an unqualified fact.

> The Primary Agent learns from permitted evidence; it does not inherit every Agent's memory.

> New evidence supersedes or disputes old claims; history is not silently rewritten.

## Knowledge object

Candidate logical object:

```text
PrimaryKnowledgeClaimV1
- schema_version
- claim_id
- subject
- predicate
- value_commitment
- provenance_root
- evidence_root
- permission_class
- confidence_class
- verification_status
- valid_from
- observed_at
- supersedes[]
- disputes[]
- expires_at?
- policy_id
```

Exact NCE/1 field IDs and canonical encoding are not locked.

A claim may reference public plaintext, encrypted/private material, content-addressed objects, execution receipts, chain state, external-source commitments, or other versioned evidence depending on policy.

## Provenance

Every admitted durable claim MUST retain enough provenance to answer, where applicable:

- who/what supplied it;
- when it was observed;
- which Agent/model/tool/runtime produced it;
- which Job/ExecutionReceipt supports it;
- which source/resource it refers to;
- what verification was performed;
- what permission allowed its use;
- whether it was transformed/summarized from earlier evidence.

Provenance is not itself proof of truth. It is the audit trail needed to evaluate a claim.

## Permission classes

Candidate classes:

```text
PUBLIC
SHARED_FOR_KNOWLEDGE
SESSION_ONLY
PRIVATE
RESTRICTED
```

Only material whose policy permits durable knowledge use may enter the shared Primary Agent knowledge system.

`PRIVATE`, `SESSION_ONLY`, or otherwise restricted memory MUST NOT become shared durable knowledge merely because the Primary Agent processed it.

Exact class IDs and inheritance rules are not locked.

## Verification status

Candidate statuses:

```text
UNVERIFIED
SOURCE_ATTESTED
CROSS_SUPPORTED
PROTOCOL_VERIFIED
DISPUTED
SUPERSEDED
RETRACTED
EXPIRED
```

These are not universal truth scores. Different claim types require different evidence.

For example, chain state can be protocol-verified under canonical chain rules while an empirical scientific claim cannot become universally true merely because several Agents repeat it.

## Confidence

Confidence MUST NOT be derived simply from the number of Agents asserting the same claim. Sybil identities can cheaply repeat misinformation.

Candidate confidence inputs include:

- independent evidence diversity;
- provenance quality;
- source history;
- applicable verification policy;
- reproducibility;
- recency where relevant;
- contradictory evidence;
- domain-specific evidence rules.

Exact scoring/ranking algorithms are deliberately not locked.

## Admission pipeline

```text
candidate information
        |
        v
permission check
        |
        v
normalize claim + provenance
        |
        v
evidence/verification policy
        |
        v
conflict + duplication analysis
        |
        v
admit / quarantine / reject
        |
        v
versioned durable knowledge
```

Economic or identity status alone MUST NOT force admission.

## Poisoning resistance

The Primary Agent MUST assume that some Agents, users, providers, websites, tools, and data feeds will be malicious, compromised, mistaken, coordinated, or low quality.

Defenses should include:

- stable provenance;
- Sybil-resistant evidence weighting where appropriate;
- independent-source diversity rather than identity counts;
- rate/resource limits;
- content/evidence deduplication;
- conflict retention;
- quarantine for suspicious/high-impact claims;
- domain-specific verification;
- revocable source trust/reputation signals;
- bounded influence from any one provider/operator;
- auditability of admission decisions.

No single Agent should be able to make a claim trusted simply by generating many supporting Agents.

## Contradictions

Contradictory claims may coexist when evidence remains unresolved.

The system SHOULD preserve the conflict rather than overwrite the older claim silently.

A response layer may select or summarize the best-supported current interpretation, but durable storage should preserve relevant provenance and disagreement.

## Supersession

Facts that change over time require temporal/version semantics.

A newer claim may `supersede` an older claim without deleting history. Supersession should indicate why the newer claim is applicable and preserve the older claim for audit/reconstruction.

Examples include software versions, network parameters, prices, availability, regulations, model capabilities, and Agent service endpoints.

## Retraction and correction

A source may retract information, or later evidence may demonstrate an earlier claim was incorrect.

Retraction/correction creates new durable state referencing the affected claim. It does not erase the historical record except where privacy/legal deletion policy explicitly requires different handling.

## Knowledge versus memory

Primary Agent **memory** may include conversational/user-specific state under privacy rules.

Primary Agent **shared knowledge** contains admitted reusable claims/evidence whose permissions allow broader use.

The two stores MUST NOT be treated as interchangeable.

A user's private conversation can influence that user's authorized session/memory without becoming global network knowledge.

## Knowledge versus model weights

Durable knowledge admission does not automatically trigger model fine-tuning/training.

Model training is a separate high-risk pipeline requiring its own dataset permissions, provenance, reproducibility, evaluation, versioning, privacy, and governance policy.

The Primary Agent can become more useful through retrieval, routing, verified knowledge, specialist discovery, and execution history before any autonomous weight update exists.

## Specialist learning

The Primary Agent may accumulate evidence about which Agents/models/profiles are useful for classes of work.

Such routing knowledge should be based on auditable execution/verification history rather than self-description alone.

A specialist's success in one domain does not imply authority in another.

## Source diversity

Multiple claims are not independent merely because they have different signatures.

Where possible, provenance analysis should detect shared upstream evidence, shared operator/domain, copied content, common model/source dependencies, or other correlation indicators.

This parallels storage-provider independence: distinct identities are not automatically independent evidence.

## High-impact claims

Policies may impose stronger admission/use requirements for claims capable of triggering economic transfers, capability grants, security changes, software updates, governance actions, or irreversible workflows.

The conversational layer may discuss uncertain information, but autonomous action should require the evidence/authority appropriate to that action.

## User-facing answers

The Primary Agent SHOULD be able to expose provenance, uncertainty, conflicts, and freshness when relevant rather than presenting all admitted claims with identical certainty.

The exact UI/citation format is not protocol-locked.

## Decentralized storage

Knowledge objects/evidence may be content-addressed and distributed under NIAHCIA storage policies. Storage providers do not decide whether a claim is true merely because they store it.

The Primary Agent's public-service storage allocation may cover eligible shared knowledge, subject to resource/abuse policy.

## Garbage collection

Not every candidate claim should live forever in hot replicated storage. Policies may distinguish durable audit history, active knowledge indexes, expired material, archival evidence, and deletable/private material.

Garbage collection MUST preserve commitments/references required for protocol auditability and must respect privacy/deletion policy.

## Security invariants

1. Private memory is not automatically shared knowledge.
2. Repetition by many identities is not proof.
3. Every durable admitted claim retains provenance.
4. Provenance is not itself truth.
5. Contradictions may coexist while unresolved.
6. Supersession preserves history.
7. Knowledge admission and model training are separate systems.
8. One provider/operator cannot obtain unlimited influence through Sybil identities.
9. High-impact autonomous actions may require stronger evidence than conversational use.
10. Storage providers do not determine truth.
11. Routing reputation is domain/context specific.
12. The Primary Agent can acknowledge uncertainty rather than manufacturing certainty.

## Required work before lock

1. canonical `PrimaryKnowledgeClaimV1` field IDs;
2. provenance/evidence object format;
3. permission-class semantics;
4. verification-status semantics;
5. conflict/supersession canonical rules;
6. content/evidence identity and deduplication;
7. source/operator correlation evidence;
8. knowledge admission policy object;
9. high-impact claim policy hooks;
10. retention/archival/deletion semantics;
11. interoperability vectors;
12. adversarial poisoning fixtures.

## Not locked

- vector database implementation;
- embedding model;
- LLM/model family;
- confidence score formula;
- reputation algorithm;
- web/search providers;
- autonomous fine-tuning;
- training dataset governance;
- moderation policy;
- user-interface citation format;
- storage replication constants.
