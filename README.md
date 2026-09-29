# Maple Proxy — development moved

This repository is archived. Proxy development, issues, and pull requests now
live in [MaplePrivacyLabs/Maple, under `proxy/`](https://github.com/MaplePrivacyLabs/Maple/tree/master/proxy).

- **Installation, configuration, and source builds:** [current proxy guide](https://github.com/MaplePrivacyLabs/Maple/blob/master/proxy/README.md).
- **Native binaries:** [Maple releases](https://github.com/MaplePrivacyLabs/Maple/releases).
- **Containers:** [current container instructions](https://github.com/MaplePrivacyLabs/Maple/blob/master/proxy/README.md#-docker-deployment).
- **New work:** [Maple issues](https://github.com/MaplePrivacyLabs/Maple/issues).

The legacy source, Git history, tags, [standalone releases](https://github.com/OpenSecretCloud/maple-proxy/releases),
and license remain here for compatibility and reference. Existing
`ghcr.io/opensecretcloud/maple-proxy` images remain historical artifacts and
receive no new publications; their `latest` tag does not track current Maple
releases. Current container publications use
`ghcr.io/mapleprivacylabs/maple-proxy`; consult the current guide before changing
an existing deployment. See the
[legacy README](https://github.com/OpenSecretCloud/maple-proxy/blob/bf3b0bb7e5cd0361ec9d2e5f9d82d7cbbb555ec4/README.md)
for the old standalone documentation.

## Preserved work

These unfinished items remain as historical references. Archival does not
mean they were implemented or resolved. Any continuation belongs in Maple,
with a link back to the original item.

- [Draft PR #48: Sigstore-enforcing SDK dependency](https://github.com/OpenSecretCloud/maple-proxy/pull/48).
- [Issue #59: RUSTSEC-2026-0285 TLS advisory](https://github.com/OpenSecretCloud/maple-proxy/issues/59).
- [Issue #51: RUSTSEC-2026-0258 h2 advisory](https://github.com/OpenSecretCloud/maple-proxy/issues/51).
- [Issue #8: audio transcription endpoint](https://github.com/OpenSecretCloud/maple-proxy/issues/8).

## License

[MIT](LICENSE).
