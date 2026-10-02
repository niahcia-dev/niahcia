# StorageProviderIndependenceV1

Status: **CANDIDATE / pre-alpha durability-risk model**

## Purpose

`StorageProviderIndependenceV1` defines how NIAHCIA should reason about storage replicas that appear to be provided by different identities but may share operators or failure domains.

The central rule is:

> distinct provider keys are not proof of distinct durability.

This specification does not attempt to prove a provider's physical location. It defines conservative evidence categories and prevents raw identity count from being confused with independently durable copies.

## Why this is necessary

An agreement requesting five providers could otherwise be satisfied by:

```text
one operator
  -> one server
     -> five provider keys
```

That provides almost no additional resilience.

Even five physical servers may share a single failure domain:

```text
five servers
  -> one rack
  -> one datacenter
  -> one network/provider
  -> one billing account/operator
```

A useful durability model therefore needs to preserve uncertainty rather than manufacture confidence from weak metadata.

## Candidate concepts

### Provider identity

The protocol identity used by a service/storage provider. Identity establishes accountability and signature ownership; it does not establish physical independence.

### Operator identity

The accountable operator controlling one or more providers where declared/known under the applicable Operator specification.

Multiple providers controlled by one operator MUST NOT be assumed fully independent merely because their provider IDs differ.

### Failure-domain claim

A versioned, signed claim about infrastructure characteristics that may be relevant to correlated failure.

Candidate dimensions include:

- operator;
- host/machine group;
- datacenter/facility;
- network/ASN;
- infrastructure/cloud provider;
- geographic region/country;
- power/facility domain where independently attestable.

Claims are evidence, not automatically truth.

### Observed correlation

Long-term independently observed behavior may reveal correlated failure even when declarations differ. Examples include providers repeatedly appearing/disappearing together or sharing strongly correlated reachability characteristics.

Observed correlation may reduce confidence; it must not be used as proof of physical co-location without sufficient evidence.

## Candidate evidence levels

For pre-alpha reasoning, provider pairs may be classified conservatively:

```text
SAME_OPERATOR
SHARED_DOMAIN
UNKNOWN
EVIDENCE_OF_SEPARATION
```

### SAME_OPERATOR

Providers are known or declared to share an operator. They are separate service endpoints but should be treated as strongly correlated for durability accounting.

### SHARED_DOMAIN

Evidence indicates a relevant common failure domain such as the same host/facility/network grouping even if provider identities differ.

### UNKNOWN

There is insufficient evidence either way. `UNKNOWN` MUST NOT be promoted to independent merely to satisfy a replication target.

### EVIDENCE_OF_SEPARATION

There is positive evidence that the providers differ across the failure-domain dimensions required by the applicable storage policy.

This means evidence of separation, not mathematical proof of independence.

## Independence policy

A future `StorageIndependencePolicy` may state which dimensions matter for a particular agreement/profile.

Example policy intent:

```text
require:
  distinct operators >= 3
  distinct network domains >= 2
  distinct facility/region evidence >= 2
```

Exact production thresholds are deliberately not locked here.

Storage agreements SHOULD eventually reference a versioned independence policy rather than relying solely on `target_provider_count`.

## Conservative counting

Before a production independence algorithm is locked:

- raw provider count MAY be reported;
- distinct known operator count MAY be reported;
- known/shared-domain warnings MAY be reported;
- unknown independence MUST remain visible;
- software MUST NOT label an object "geographically redundant" or "independently replicated" solely from unique provider keys.

Example telemetry:

```text
providers_observed: 5
distinct_known_operators: 2
network_domains_observed: 2
independence_unknown: 2
shared_domain_warnings: 1
```

This is more honest than reducing uncertain infrastructure to one unsupported score.

## No mandatory central identity provider

NIAHCIA MUST NOT require one developer-operated registry, KYC provider, cloud API, IP geolocation vendor, or datacenter authority to decide storage eligibility network-wide.

Independent evidence sources and versioned policies should be composable. A future market may choose stricter provider requirements for high-value data without imposing those requirements on every object.

## Sybil resistance

No single signal solves storage Sybil resistance.

Candidate signals may include:

- Operator linkage;
- economic bonds/locked obligations;
- long-lived challenge history;
- network-path/ASN evidence;
- independent observers;
- service history;
- infrastructure attestations where optional;
- correlated-failure history;
- inability to satisfy simultaneous unpredictable retrieval/challenge obligations from one constrained resource.

Every signal has limitations and must be threat-modeled before affecting compensation.

Economic identity cost can make Sybil creation more expensive but does not prove physical independence.

## Simultaneous challenges

One useful future technique is to challenge nominally independent providers concurrently with deadlines that make serial retrieval from a single hidden backing copy difficult.

This can raise the cost of pretending that one copy is many copies, but it is not sufficient alone: an operator may maintain caches, fast internal links, or multiple endpoints around one underlying failure domain.

Concurrent challenge semantics therefore belong as one evidence source rather than a universal proof of independence.

## Privacy

Provider-independence evidence must not require public disclosure of exact street addresses, personal identity, rack coordinates, or other unnecessary sensitive infrastructure information.

Policies should prefer the minimum evidence needed to distinguish relevant failure domains.

## Relationship to StorageHealthV1

StorageHealth must distinguish quantity from confidence.

An object may have five responding provider identities and still be `DEGRADED` under a future policy if those providers do not satisfy the agreement's required independent durability evidence.

Until that policy is deterministic and locked, this distinction remains observational telemetry and must not create incompatible settlement state.

## Relationship to StorageAgreementV1

A future revision should allow `StorageAgreementV1` to bind an `independence_policy_id`.

This candidate does not modify the existing candidate schema yet. That change should occur together with NCE/1 field allocation and canonical vectors rather than casually changing the agreement hash surface.

## Security invariants

1. Unique provider keys do not prove independent replicas.
2. Operator identity does not prove physical topology.
3. Self-declared geography/facility data is not trusted by default.
4. Unknown independence remains unknown.
5. No centralized identity/geolocation service is protocol authority.
6. Independence evidence never grants chain-consensus authority.
7. Policies must avoid requiring unnecessary disclosure of sensitive provider location data.
8. A durability claim must identify which versioned independence policy/evidence semantics produced it.

## Required vectors before lock

1. five provider IDs / one operator;
2. multiple operators / one known shared failure domain;
3. unknown-domain providers remain UNKNOWN;
4. mixed evidence with a policy threshold;
5. provider evidence expiry/change case;
6. simultaneous-challenge evidence case;
7. contradictory self-declared vs observed evidence;
8. policy-version mismatch rejection;
9. deterministic conservative-count fixture;
10. cross-implementation independence-evidence fixture.

## Not locked in this candidate

- production independence score;
- required operator bond;
- mandatory infrastructure attestation;
- geolocation mechanism;
- exact ASN/network proof mechanism;
- reputation weighting;
- agreement thresholds;
- settlement/reward effects;
- simultaneous-challenge parameters.
