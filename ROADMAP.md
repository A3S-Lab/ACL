# A3S ACL Roadmap

**Status as of 2026-09-10.**

A3S ACL is the only product configuration language. Parsing and generation use
`a3s-acl` only.

## A3S Cloud substrate obligations

| Priority | This repository must deliver | Forbidden |
| --- | --- | --- |
| Wave 0 / `COMP` | Byte-stable digests; schema stability and deprecation hooks | TOML/HCL product-config parsers; hashing non-canonical ACL |
| Consumers | Conformance kits for Cloud, Runtime, Flow, Gateway, Use | Owning tenant policy, authorization, or placement |

Portfolio detail:
[foundations-and-execution.md](https://github.com/A3S-Lab/Cloud/blob/main/docs/project-roadmaps/foundations-and-execution.md).

Monorepo index:
[cloud-substrate-dependency-roadmap.md](https://github.com/A3S-Lab/a3s/blob/main/docs/cloud-substrate-dependency-roadmap.md).
