# Sekha Enterprise Commercial License

**Version 1.0**  
**Effective Date:** January 1, 2026  
**Last Updated:** January 29, 2026

---

## 1. OVERVIEW

This document outlines the commercial licensing terms for **Sekha Enterprise**, a commercial software license alternative to the AGPL-3.0 free license. Organizations that do not qualify for or prefer not to comply with AGPL-3.0 terms may obtain a commercial license from Sekha AI, Inc.

### 1.1 License Grant

Subject to payment of applicable fees and compliance with the terms herein, **Sekha AI, Inc.** ("Licensor") grants you a non-exclusive, non-transferable license to:

- Install and use Sekha software on your premises or cloud infrastructure
- Modify Sekha software for your internal business purposes
- Deploy Sekha internally without publishing modifications
- Build commercial products incorporating Sekha without open-sourcing derivative works

### 1.2 Scope

This commercial license covers:

- **Sekha Controller** (core memory orchestration engine)
- **LLM Bridge** (language model integration)
- **Sekha Proxy** (universal AI interface middleware)
- **All official SDKs** (Python, JavaScript, Rust)
- Official plugins and extensions

---

## 2. ELIGIBILITY & PRICING

### 2.1 License Tiers

Commercial licenses are available in the following tiers based on organization size and API call volume:

#### Startup Tier
- **Organization Size:** 51-200 employees
- **Annual Fee:** $5,000 USD
- **Included API Calls:** 10 million/year
- **Overage Rate:** $0.001 per call
- **Support:** Email support (48-hour response)
- **Deployment:** Self-hosted or cloud

#### Business Tier
- **Organization Size:** 201-1,000 employees
- **Annual Fee:** $25,000 USD
- **Included API Calls:** 100 million/year
- **Overage Rate:** $0.0005 per call
- **Support:** Email support (24-hour response) + monthly architecture consultation
- **Deployment:** Self-hosted, cloud, or hybrid

#### Enterprise Tier
- **Organization Size:** 1,000+ employees
- **Annual Fee:** $100,000 USD
- **Included API Calls:** 500 million/year
- **Overage Rate:** $0.0002 per call
- **Support:** 24/7 phone/email support (SLA-backed, 4-hour response)
- **Deployment:** All models + on-premise deployment assistance
- **Additional:** Security audits, compliance documentation

#### AI Vendor Tier
- **Applies To:** OpenAI, Anthropic, Google, and comparable AI service providers
- **Fee Structure:** Custom negotiation with revenue-share component
- **Typical Terms:** 0.1-0.8% of Sekha-attributable revenue
- **Support:** Dedicated technical account manager
- **Deployment:** Unlimited API calls

### 2.2 Employee Count Definition

"Employees" includes:
- Full-time employees
- Part-time employees (0.5+ FTE)
- Contractors on payroll for 90+ consecutive days
- Subsidiary and affiliate employees (consolidated headcount)

**Exclusions:** Temporary workers, consultants not on payroll, acquired companies during first 12 months of integration

### 2.3 API Call Limits

"API calls" are counted as:
- Each HTTP request to Sekha REST API endpoints
- Each MCP protocol invocation via LLM context
- Each SDK method call that triggers backend processing

**Not counted:** Internal function calls, webhook deliveries, cache hits

---

## 3. RESTRICTIONS & PERMITTED USE

### 3.1 Permitted Uses

You may:

- ✅ Install Sekha on your infrastructure (on-premise or cloud)
- ✅ Modify Sekha source code for internal purposes
- ✅ Integrate Sekha into commercial products without open-sourcing
- ✅ Deploy Sekha as a service to your customers (SaaS)
- ✅ Redistribute modified Sekha in compiled form (binaries, Docker images)
- ✅ Use Sekha for commercial purposes without revenue sharing
- ✅ Sublicense Sekha to subsidiaries (with appropriate licensing tier)

### 3.2 Restricted Uses

You may **not**:

- ❌ Sell, sublicense, or distribute Sekha source code to third parties
- ❌ Offer Sekha as a hosted service without a commercial license
- ❌ Remove or modify copyright notices, license headers, or trademark attribution
- ❌ Use Sekha in violation of applicable laws or regulations
- ❌ Rent, lease, or timeshare Sekha to customers without appropriate licensing
- ❌ Transfer this license without Licensor's written consent
- ❌ Circumvent license key validation, telemetry, or usage tracking

### 3.3 Network Use & Distribution

**Network Access = Distribution**: If your Sekha deployment is accessible to external users (customers, partners, APIs), it is considered "distribution" and requires either:
1. This commercial license, OR
2. AGPL-3.0 with source code publication

**Examples:**

| Scenario | License Required |
|----------|------------------|
| Internal-only deployment for your employees | Commercial |
| SaaS platform powered by Sekha | Commercial |
| Customer-facing API returning Sekha results | Commercial |
| Embedded in software you distribute | Commercial (or AGPL-3.0 + open-source) |
| Integrated in a device you sell | Commercial |

---

## 4. PAYMENT TERMS

### 4.1 Invoice & Payment

- **Billing Cycle:** Annual, invoiced in advance
- **Payment Method:** ACH, wire transfer, or credit card
- **Terms:** Net 30 days from invoice date
- **Currency:** USD
- **Late Fees:** 1.5% per month on overdue amounts

### 4.2 Usage Overages

- **Reporting:** Sekha reports usage monthly
- **Billing:** Overage charges invoiced the following month
- **Transparency:** Usage dashboard available at license.sekha.dev

### 4.3 Price Adjustments

- **Annual Increases:** Up to 10% per year, with 60 days' notice
- **Volume Adjustments:** If your organization size changes tier, reconciliation occurs at next renewal
- **Prepayment Discounts:** 10% discount for 2-year prepayment

### 4.4 Startup Discounts

Qualifying startups receive **50% off Startup Tier** for first 12 months:
- Company founded within past 24 months, AND
- Less than $1 million annual revenue
- Must provide supporting documentation

---

## 5. SUPPORT & SERVICES

### 5.1 Support Tiers by License Level

| Tier | Email | Response SLA | Phone | Consulting |
|------|-------|-------------|-------|-----------|
| Startup | ✅ | 48 hours | ❌ | — |
| Business | ✅ | 24 hours | ❌ | 4 hrs/month |
| Enterprise | ✅ | 4 hours | ✅ 24/7 | Unlimited |
| AI Vendor | ✅ | 1 hour | ✅ 24/7 | Dedicated AM |

### 5.2 What's Included

**All Tiers:**
- Access to product roadmap
- Security patches and critical updates
- Usage reporting and analytics
- License management portal

**Business & Enterprise:**
- Architecture review consultations
- Custom deployment configurations
- Integration support for LLM providers

**Enterprise Only:**
- Compliance documentation (HIPAA, SOC2, PCI-DSS)
- Custom feature development
- Penetration testing support
- Dedicated security liaison

### 5.3 What's Not Included

Support does **not** cover:
- Your application code or integrations
- Third-party libraries or dependencies
- Custom feature development (unless Enterprise tier)
- General LLM tuning or prompt engineering

---

## 6. INTELLECTUAL PROPERTY

### 6.1 Sekha IP

Licensor retains all intellectual property rights in Sekha, including:
- Source code and object code
- Documentation and specifications
- Designs, algorithms, and know-how
- Trademark and branding

This license does **not** grant ownership or intellectual property rights, only usage rights.

### 6.2 Your Modifications

You own intellectual property rights to modifications you create. However:
- You grant Licensor a perpetual, royalty-free license to use improvements you contribute
- You agree not to claim trademark rights on Sekha
- Derivative works remain subject to this license

### 6.3 Feedback

Any feedback, suggestions, or feature requests you provide may be used by Licensor without compensation or attribution.

---

## 7. WARRANTY & LIABILITY DISCLAIMERS

### 7.1 Disclaimer of Warranties

SEKHA IS PROVIDED "AS-IS" WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO:

- Warranties of merchantability or fitness for a particular purpose
- Warranties that Sekha will meet your requirements
- Warranties that Sekha will be uninterrupted or error-free
- Warranties regarding the security or reliability of data storage

### 7.2 Limitation of Liability

**NEITHER PARTY SHALL BE LIABLE FOR:**

- Indirect, incidental, consequential, special, or punitive damages
- Loss of profits, revenue, data, or business opportunity
- Claims arising from third-party services or LLM providers

**MAXIMUM LIABILITY:** The total amount paid under this license in the 12 months preceding the claim, or $10,000 USD, whichever is greater.

### 7.3 Exceptions

Limitations do not apply to:
- Breach of confidentiality obligations
- Infringement of intellectual property rights
- Indemnification obligations
- Either party's gross negligence or willful misconduct

---

## 8. CONFIDENTIALITY

### 8.1 Confidential Information

Each party agrees to maintain confidentiality of:
- License key and deployment credentials
- Customer usage data and API statistics
- Security vulnerability information (pre-disclosure)
- Technical roadmap details

### 8.2 Permitted Disclosures

You may disclose Sekha's existence to:
- Your employees and contractors (under NDA)
- Your legal and financial advisors
- Your customers (as required to explain your service)

You **may not** publicly disclose:
- Pricing or financial terms
- Security vulnerabilities
- Pre-release features

---

## 9. COMPLIANCE & AUDIT RIGHTS

### 9.1 Compliance Verification

You agree to:
- Maintain accurate records of Sekha deployment and usage
- Use only the latest stable version (security patches mandatory)
- Not circumvent license key validation or telemetry
- Notify Licensor of security breaches within 48 hours

### 9.2 Audit Rights (Enterprise Tier Only)

Licensor reserves the right to audit:
- Deployment architecture and security controls
- API call logging and usage records
- License key validity and version compliance

**Audit Procedure:**
- 30 days' advance notice
- Conducted during business hours
- May be performed by Licensor or authorized third-party
- Cost borne by Licensor unless material non-compliance found
- Maximum one audit per calendar year

### 9.3 Remediation

If audit reveals non-compliance:
- You have 30 days to remediate
- Licensor may bill overage costs retroactively
- Continued non-compliance may result in license termination

---

## 10. TERM & TERMINATION

### 10.1 License Term

- **Initial Term:** One (1) year from effective date
- **Renewal:** Automatic renewal for successive one-year terms unless either party provides 60 days' non-renewal notice
- **Early Termination:** Either party may terminate for material breach with 30 days' cure period

### 10.2 Termination for Cause

Licensor may terminate immediately without cure period if:
- You breach IP restrictions (Section 3.2)
- You circumvent license validation or security measures
- You fail to pay for 60+ days
- You become insolvent or file bankruptcy
- You materially breach confidentiality obligations (Section 8)

### 10.3 Effect of Termination

Upon termination:
- All licenses granted herein immediately cease
- You must stop using Sekha and destroy all copies
- You may retain data exported from Sekha
- Sections 6-12 survive termination indefinitely

### 10.4 Post-Termination Options

If license is terminated and you wish to continue using Sekha:
1. **Switch to AGPL-3.0:** Open-source your modifications, comply with copyleft
2. **Obtain New Commercial License:** Submit new application (re-qualification required)

---

## 11. MODIFICATIONS TO THIS AGREEMENT

### 11.1 Updates

Licensor reserves the right to modify these terms with 60 days' notice. Changes do not apply to current subscription period—only to renewals.

### 11.2 Material Changes

If changes are materially adverse, you may terminate without penalty during the notice period.

### 11.3 Incorporation of Updates

Your continued use after the notice period constitutes acceptance of updated terms.

---

## 12. GENERAL PROVISIONS

### 12.1 Governing Law

This Agreement shall be governed by and construed in accordance with the laws of **Delaware, USA**, without regard to its conflict of law principles.

### 12.2 Jurisdiction & Venue

Exclusive jurisdiction and venue for disputes shall be in the **Delaware Court of Chancery** or, if subject matter jurisdiction does not exist, the federal courts in Delaware.

### 12.3 Entire Agreement

This License Agreement, together with the AGPL-3.0 terms, constitutes the entire agreement regarding Sekha and supersedes all prior negotiations, representations, and agreements.

### 12.4 Severability

If any provision is found unenforceable, it shall be modified to the minimum extent necessary to make it enforceable, and all other provisions remain in full effect.

### 12.5 Waiver

Failure to enforce any right does not constitute waiver of that right. No waiver is effective unless in writing and signed by Licensor.

### 12.6 Assignment

You **may not** assign this license without Licensor's written consent. Assignments in violation of this clause are void. Licensor may assign to successors or acquirers without notice.

### 12.7 Counterparts

This Agreement may be executed in multiple counterparts, each deemed an original, all constituting one instrument.

---

## 13. CONTACT & LICENSING INQUIRIES

For commercial licensing questions, API usage clarification, or to request a custom tier:

**Email:** hello@sekha.ai  
**Licensing Portal:** license.sekha.ai  
**Address:** Sekha AI, Inc. | Delaware, USA

---

## APPENDIX A: LICENSE KEY FORMAT & VALIDATION

### A.1 License Key Structure

```
sekha_enterprise_[tier]_[org-id]_[signature]
```

**Example:**
```
sekha_enterprise_startup_org-12345_eyJhbGciOiJFUzI1NiJ9.eyJvcmdfaWQiOiJvcmctMTIzNDUiLCJ0aWVyIjoic3RhcnR1cCIsImV4cCI6MTc0NjUwNDAwMH0.signature
```

### A.2 Validation

Sekha automatically validates:
- Key signature via public JWKS (no phone-home required)
- Expiration date
- Organization ID against deployment

### A.3 Offline Validation

Keys are verified cryptographically without internet connectivity—you own your license fully.

---

## APPENDIX B: API CALL COUNTING METHODOLOGY

### B.1 Countable Calls

Each of these counts as one API call:

```
POST /api/v1/memory/store
POST /api/v1/memory/retrieve
POST /api/v1/memory/search
POST /api/v1/context/assemble
POST /api/v1/llm/query
GET /api/v1/stats
GET /api/v1/export
```

### B.2 Non-Countable

These **do not** count:

```
GET /api/v1/health
GET /api/v1/version
Cache hits
Internal function calls
Webhook deliveries
Error responses (4xx, 5xx)
```

### B.3 Metering

- Real-time usage dashboard available
- Monthly billing report 2 days after month-end
- Usage data retention: 24 months

---

## APPENDIX C: COMPLIANCE DOCUMENTATION AVAILABILITY

### Enterprise Tier Compliance Documents

Upon request, Licensor provides:

- [ ] HIPAA Business Associate Agreement (BAA)
- [ ] SOC 2 Type II audit report
- [ ] ISO 27001 compliance statement
- [ ] GDPR Data Processing Agreement (DPA)
- [ ] CCPA compliance documentation
- [ ] Encryption and key management specifications
- [ ] Incident response procedures

**Request Process:** Email hello@sekha.ai with compliance requirements, 5-7 business days for provision.

---

**END OF COMMERCIAL LICENSE AGREEMENT**