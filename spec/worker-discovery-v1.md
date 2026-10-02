# Worker Discovery V1

Status: **CANDIDATE / pre-alpha decentralized worker advertisement discovery**

## Purpose

Worker Discovery V1 defines how wallets/clients obtain current signed WorkerAdvertisement records without relying on niahcia.com or another permanent central directory.

The central rule is:

> discovery may use multiple replaceable sources, but only signed, unexpired worker advertisements are authoritative about a worker's advertised state.

## Advertisement authority

A discovery source is not trusted to invent worker properties.

A wallet validates each WorkerAdvertisement independently, including:

- worker signature;
- worker_id / operator_id binding;
- advertisement expiry;
- supported model/profile claims;
- pricing references;
- endpoint descriptor;
- status;
- advertisement sequence or freshness rules.

A relay that serves stale, malformed, or fabricated data cannot create a valid worker advertisement without the worker's signing authority.

## Discovery sources

Wallets MAY obtain advertisements from multiple mechanisms, including:

- direct peer gossip;
- bootstrap peers;
- cached peer lists;
- protocol relays;
- local configuration;
- DNS/bootstrap hints;
- niahcia.com or another website;
- future DHT/discovery mechanisms.

No single source is authoritative.

niahcia.com may be convenient, but a conforming wallet MUST be able to use other sources.

## First-milestone discovery

The first implementation SHOULD use the simplest decentralized design that proves replaceability:

```text
wallet
  -> one or more bootstrap peers
  -> receive signed WorkerAdvertisements
  -> verify locally
  -> cache until expiry
  -> learn additional peers/workers
```

This is sufficient for devnet. A DHT is not required for the first milestone.

## Gossip

Peers MAY gossip signed advertisements they did not create.

Relays MUST forward the original signed advertisement unchanged.

A wallet may deduplicate by worker_id plus advertisement sequence/hash.

Expired advertisements MUST NOT be considered eligible even if still cached or repeatedly gossiped.

## Freshness

WorkerAdvertisement should include either a monotonic worker advertisement sequence, a unique signed advertisement identifier, or another replay-safe freshness mechanism.

When two valid advertisements exist for the same worker, wallets prefer the newest valid advertisement under the defined worker advertisement ordering.

A malicious relay cannot keep a worker permanently ACTIVE merely by replaying an expired advertisement.

## Endpoint reachability

A syntactically valid advertisement does not prove the endpoint is reachable.

Wallets MAY perform lightweight reachability/handshake probes before considering a worker ready for selection.

Failure to connect may temporarily suppress that worker locally without creating global slashing evidence.

## Privacy

A wallet SHOULD be able to discover workers without revealing the user's prompt, conversation identity, or payment account.

Discovery queries may reveal model/profile interest depending on mechanism.

For stronger privacy, wallets may fetch broader advertisement sets and filter locally rather than querying a centralized service for one exact model.

## Sybil boundary

Discovery itself does not solve worker Sybil resistance.

WorkerSelectionV1, operator identity, service bonds/accountability policy, verification diversity, and observed behavior handle the consequences of many worker identities.

Discovery relays MUST NOT gain authority merely by ranking or ordering results.

## Caching

Wallets MAY cache advertisements locally until expiry.

Cached data should preserve the original signed bytes or canonical signed object so it can be revalidated after restart.

A wallet MUST refresh before opening a new ComputeSession when required advertisement data has expired.

## Bootstrap failure

The network must tolerate loss of the original bootstrap host.

Wallets SHOULD retain learned peer addresses and support multiple bootstrap hints.

Bootstrap peers help establish initial connectivity; they are not schedulers and do not decide which worker the wallet selects.

## Invariants

1. niahcia.com is not an authoritative worker directory.
2. Discovery sources cannot forge valid WorkerAdvertisements.
3. Wallets validate advertisements locally.
4. Expired advertisements are ineligible.
5. Relays do not gain scheduling authority.
6. Workers can be discovered through more than one source.
7. Worker discovery does not require decentralized storage.
8. Discovery does not reveal wallet root/spending keys.
9. Bootstrap infrastructure is replaceable.
10. A DHT is not required for the first end-to-end milestone.

## First milestone

Implement:

1. two or more bootstrap/discovery peers;
2. signed expiring WorkerAdvertisements;
3. advertisement gossip/relay;
4. local wallet validation and caching;
5. endpoint probe;
6. local eligibility filter;
7. local worker selection;
8. demonstration that shutting down niahcia.com does not prevent wallet-to-worker discovery once alternate peers are available.
