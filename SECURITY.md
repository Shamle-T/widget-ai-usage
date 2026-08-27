# Security Policy

## Reporting a vulnerability

Until a public security contact is configured, do not publish sensitive exploit details in a public issue. Contact the future project maintainer through the repository’s private reporting channel once configured.

Include affected version, platform, reproduction steps, impact, and any proof of concept that does not expose credentials, source code, or private provider data.

## Security commitments

- LimitLens will not scrape browser cookies or browser sessions.
- LimitLens will not read provider tokens from local configuration or environment variables.
- LimitLens will not send AI prompts or perform model inference to estimate quota.
- Provider adapters must use documented safe mechanisms and return unavailable when none exists.
- Do not add WebView, arbitrary shell, or broad filesystem access without a documented security review.

See [docs/security.md](docs/security.md) for the current review record.
