# Diagnostics

Use diagnostics to inspect extraction decisions.

Diagnostics are for development, fixture work, and bug reports. They explain
which candidates were considered, which root was selected, and why an extraction
was accepted or downgraded to a best attempt.

```rust
use lectito::{extract_with_diagnostics, ReadabilityOptions};

let report = extract_with_diagnostics(html, base_url, &ReadabilityOptions::default())?;
println!("{:?}", report.diagnostics.outcome);
```

`ExtractionReport` contains:

- `article`: the extracted article, if found
- `diagnostics`: details about attempts and candidate selection

Outcomes:

| Outcome | Meaning |
| --- | --- |
| `Accepted` | An attempt met `char_threshold`. |
| `BestAttempt` | No attempt met the threshold, but non-empty content was found. |
| `NoContent` | No useful content was found. |

Each attempt records:

- cleanup flags
- candidate count
- top candidates
- entry points
- selected root
- cleanup counts
- recovery counts
- extracted text length

When a site profile or code extractor matches, diagnostics include `site_rule`.
That record reports the matched profile or extractor, whether it was bundled,
which roots were selected, how many removals ran, whether the result met
`char_threshold`, and any fallback reason.

Start with `outcome`, `selected_root`, and `text_len`. If the selected root is
wrong, inspect the candidate list. If the root is right but output is noisy,
inspect cleanup counts and preserved classes.

CLI diagnostics:

```sh
lectito parse article.html --diagnostic-format pretty
lectito parse article.html --diagnostic-format json
```
