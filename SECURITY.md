# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.2.x   | :white_check_mark: |

## Reporting a Vulnerability

**Please DO NOT file a public issue for security vulnerabilities.**

Email: security@sekha.dev

We will respond within 48 hours with an initial assessment.

### What to Include

- Description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fixes (if any)

### Disclosure Policy

- We will acknowledge your email within 48 hours
- We will provide a detailed response within 7 days
- We will work on a fix and release timeline
- We will credit you in the security advisory (unless you prefer to remain anonymous)

## Security Features

- Bearer token authentication required for all API endpoints
- Rate limiting (100 req/s, burst 200)
- CORS protection
- Input validation via Validator crate
- No external dependencies in production binary
- Regular security audits via cargo-deny and cargo-audit