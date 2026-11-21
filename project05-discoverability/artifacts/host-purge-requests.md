# Private provider purge requests — drafts, not sent

These requests serve the remaining privacy-cleanup gate. They intentionally do
not repeat the confidential customer identity. Submission to either provider
requires Duncan's explicit approval; do not post a public community issue.

## GitHub Support

Route: [GitHub Support](https://support.github.com/) through the private portal
linked by [GitHub's sensitive-data removal instructions](https://docs.github.com/en/authentication/keeping-your-account-and-data-secure/removing-sensitive-data-from-a-repository).

Subject: Private request: purge retained pre-rewrite Git objects for oxur/textrynum

Hi GitHub Support,

I maintain https://github.com/oxur/textrynum. A confidential customer identity
was included in source, fixtures and planning documents. I've rewritten the
history and replaced the affected branches and tags. Fresh fetches of every
advertised ref are clean, but the old commit below is still retrievable by hash.

Could you run the necessary server-side collection and remove cached views or
retained references that keep this pre-rewrite content accessible? This is
confidential client-identifying information, rather than a credential that can
be rotated. Please assess it under your sensitive-data removal process.

- Old main commit confirmed retrievable: `f19bb3b9bc00295dea8eabb40247b6eb9e6fabdf`
- Rewritten main at verification: `856af87561c7e53379ec9094c69a01211d3e5283`
- Affected branches/tags were replaced with checked, atomic old-ref leases.
- The two advertised PR heads were checked; neither contained private-name objects. I have not identified a PR requiring deletion. Please preserve PR conversations where possible.
- No LFS pointer blobs were found in the retained local object population; no orphaned LFS removal was identified.
- First rewritten root for public source history, derived from the commit map: `53f852e9e450cb5e9c06db449103a0a97bd13fa8`. This rewrite also removed historical commit signatures.

A concrete affected source location in the old main tree is
`crates/ecl-stages/src/field_map.rs`. The current version is anonymized.

Please keep this request private and let me know when the old objects and cached
views have been purged so I can verify that they are no longer accessible.

Thanks,
Duncan

## Codeberg

To: contact@codeberg.org, the private support route in
[Codeberg's contact documentation](https://docs.codeberg.org/contact/).
The public community issue tracker is inappropriate for disclosing this material.

Subject: Private request: collect retained pre-rewrite objects for oxur/ecl

Hi Codeberg team,

I maintain https://codeberg.org/oxur/ecl. A confidential customer identity was
included in source, fixtures and planning documents. I've rewritten the affected
history and pushed the replacement branches and tags. Fresh fetches of every
advertised ref are clean, but the old commit below remains retrievable by hash.

Could you expire obsolete retention and run repository garbage collection, and
remove any cached views that keep this pre-rewrite content available? I understand
collection normally runs periodically; I'm asking whether it can be completed
for this repository because the retained material identifies a private customer.

- Old main commit confirmed retrievable: `7a9dead8b608d959d592d393e62094872a4e2a8e`
- Rewritten main at verification: `87af29b820e495afe2dbe1b0ad5ce5173d59fa43`
- All advertised refs were freshly fetched and checked after the rewrite.
- A concrete affected old source path is `crates/ecl-stages/src/field_map.rs`.

Please handle this privately. I have avoided repeating the customer's identity
in this message; the old commit and source path above identify the affected data.
Please let me know when collection is complete so I can repeat the access check.

Thanks,
Duncan
