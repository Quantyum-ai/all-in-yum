# PRP: OAuth/IAM Migration Research & Context Gathering

## Title
Comprehensive Provider OAuth/IAM Capability Assessment & Architecture Research for all-in-yum

## Objective
Conduct thorough research to gather all necessary context for planning an auth migration of the all-in-yum CLI from provider API keys toward a “no manual API keys” posture using OAuth access tokens and/or IAM-equivalent mechanisms (cloud IAM, request signing, managed identities, workload identity). This research will inform the subsequent implementation PRP by verifying provider capabilities, analyzing reference implementations (including coding CLIs), and identifying proven patterns for auth brokers and enterprise SSO integration.

## Scope

### In Scope
1. **Repo Ground Truth (CRITICAL)**: Prove, with `path:line` citations, how all-in-yum authenticates today (direct provider APIs, key storage backends, config surface).
2. **Direct Provider OAuth Capability Verification**: Determine if xAI, Anthropic, Google Gemini, and OpenAI support OAuth access tokens for their AI model APIs (not just user account OAuth).
3. **Cloud IAM Alternatives (CRITICAL)**: Determine whether cloud-hosted equivalents (e.g., AWS Bedrock, Google Vertex AI, Azure OpenAI, OCI GenAI) provide an IAM-style path that avoids manual provider API keys, and what adapter changes that implies.
4. **Reference Implementation Analysis (Coding CLIs)**:
   - AutoMaker: multi-provider token-based authentication patterns.
   - grok-cli: how the Grok coding CLI authenticates and stores tokens/keys (if applicable).
5. **Auth Broker Pattern Research**: Identify proven patterns for auth brokers/gateways when providers lack direct OAuth/IAM.
6. **SSO Integration Research**: Enterprise CLI authentication patterns (OIDC device flow, PKCE, SAML bridges).
7. **Token Management Best Practices**: Security patterns for token storage, refresh, revocation, and redaction.

### Non-Goals
- Writing implementation code
- Making architectural decisions (research team gathers facts; planning team decides)
- Testing provider APIs directly

## Evidence Standards (Must Follow)

### 1) Repo claims require `path:line`
Any statement about all-in-yum behavior MUST cite `path:line` in this repo. If a claim cannot be backed by code, mark it **UNKNOWN** and list the file(s) needed to prove it.

**Examples (must be re-verified by the research team, do not blindly copy):**
- Grok direct API uses bearer API key: `crates/aiy-adapter-grok/src/client.rs:150`
- Claude direct API uses `x-api-key`: `crates/aiy-adapter-claude/src/client.rs:172`
- Gemini direct API uses `?key=` query param: `crates/aiy-adapter-gemini/src/client.rs:125`
- Codex/OpenAI direct API uses bearer API key: `crates/aiy-adapter-codex/src/client.rs:153`
- `CredentialBackend::SecretManager` exists but is unimplemented: `crates/aiy-core/src/security/credential_manager.rs:238`
- Default base URLs target direct provider endpoints: `crates/aiy-core/src/config/pipeline.rs:85`

### 2) Provider capability claims require official documentation
Any statement like “Provider X supports OAuth for API access” MUST include at least one official vendor documentation link to the exact product/API being discussed.

If a claim cannot be supported with official docs, mark it **UNKNOWN** and list what documentation would settle it.

### 3) Do not conflate token types
Be explicit whether a “Bearer” token is:
- An OAuth access token (time-limited, refreshable via OAuth), or
- A provider API key/PAT that happens to be sent in a Bearer header.

## Research Deliverables

The research team must produce a comprehensive markdown report (`docs/oauth-research-findings.md`) containing the following sections:

### Section 0: Repo Ground Truth (Required)

**Objective**: Provide a verified map of current authentication and credential flows in all-in-yum so downstream planning is grounded in code, not assumptions.

**Required Output (with `path:line` citations):**
- Current auth mechanisms for each direct adapter:
  - Grok/xAI adapter: how it retrieves credentials and attaches auth to HTTP requests.
  - Claude/Anthropic adapter: same.
  - Gemini/Google adapter: same.
  - Codex/OpenAI adapter: same.
- Credential storage options and limitations:
  - `CredentialBackend` variants and what is implemented vs planned.
  - How CLI selects backend (config/env/commands).
- Config surfaces relevant to auth:
  - Base URL configuration, provider naming conventions (e.g., “xai”, “anthropic”, “google”, “openai”).
  - Any existing `http` feature gating that changes behavior.

**Acceptance check**: Every bullet above has at least one `path:line` citation.

### Section 1: Provider Capability Matrix (Direct Provider APIs)

For each provider (xAI/Grok, Anthropic/Claude, Google/Gemini, OpenAI/Codex), research and document:

**Required Information**:
- **OAuth Support Status**: Does this provider support OAuth access tokens for API access to the relevant model API? (YES/NO/PARTIAL)
- **Documentation URL**: Official authentication documentation link for that exact API/product
- **Authentication Flow**: What flow is supported? (client credentials, device code, authorization code + PKCE, etc.)
- **Token Format**: JWT? Opaque token? Bearer token?
- **Token Lifetime**: Access token expiration (typical duration)
- **Refresh Support**: Can tokens be refreshed? How?
- **Scope/Audience**: What OAuth scopes or audience values are needed?
- **Request Attachment**: How is the token attached to API requests? (Authorization header, query param, custom header?)
- **API Base URL**: Separate OAuth endpoint vs API endpoint?
- **Client Registration**: How does a CLI app register as an OAuth client?
- **Revocation**: How are tokens revoked?

**Important**: Also document if the provider supports only API keys/PATs for the direct API, even if the company has OAuth for other products.

**Research Strategy**:
1. Search official provider documentation:
   - "xAI Grok OAuth API authentication"
   - "Anthropic Claude API OAuth"
   - "Google Gemini API OAuth authentication"
   - "OpenAI API OAuth authentication"
2. Look for developer guides, quickstart tutorials
3. Check GitHub repos for official SDK authentication examples
4. Search for "provider-name API authentication best practices"

**Output Format**:
```markdown
## Provider: xAI (Grok)

**OAuth Support Status**: [YES/NO/PARTIAL]

**Summary**: [2-3 sentence summary of findings]

**Details**:
- Documentation URL: [link]
- Authentication Flow: [flow name]
- Token Format: [format]
- Token Lifetime: [duration]
- Refresh Support: [yes/no + details]
- Scope/Audience: [values]
- Request Attachment: [method]
- API Base URL: [url]
- Client Registration: [process]
- Revocation: [method]

**Code Example** (if available):
```[language]
[example code from official docs]
```

**Assessment**: [Can all-in-yum use this? What are the limitations? What's missing?]

---

[Repeat for Anthropic, Google, OpenAI]
```

### Section 2: Cloud IAM / Enterprise Alternatives Matrix (Required)

**Objective**: Determine whether we can achieve “no manual API keys” by routing through cloud-hosted equivalents that support IAM-style auth (signed requests / managed identities / workload identity), and what it would cost architecturally (new adapters/endpoints).

For each “cloud alternative”, document (with official docs):
- **Is the target model/provider available on that platform?** (YES/NO/UNKNOWN)
- **Auth mechanism**: SigV4 / Entra ID (AAD) / Google service accounts / instance/resource principals / workload identity federation.
- **CLI credential acquisition**: SSO flow, managed identity, workload identity, or static keys (and which is recommended).
- **Request attachment**: header/signing approach and required endpoints.
- **Adapter impact**: new adapter vs reuse existing, base URL changes, request/response schema changes.

**Hard rule**: Do not claim “OCI GenAI hosts Grok” or “Vertex hosts Claude” unless official docs explicitly confirm that model/provider pairing.

Suggested cloud alternatives to evaluate (expand as needed):
- AWS Bedrock ↔ Claude (Anthropic)
- Google Vertex AI ↔ Gemini (Google)
- Azure OpenAI ↔ OpenAI models
- OCI GenAI ↔ any xAI/Grok availability (only if confirmed)

### Section 3: Reference Implementations (Coding CLIs)

**Objective**: Analyze known multi-provider CLIs and coding CLIs to extract proven patterns for token acquisition, caching, refresh, and secure storage.

**Required repos**
- AutoMaker: https://github.com/AutoMaker-Org/automaker
- grok-cli: https://github.com/airplne/grok-cli

**Required Output**:
- Key files analyzed (`path:line` if local checkout is available; otherwise include commit SHA + GitHub permalink).
- What auth types they use (OAuth access tokens vs API keys vs something else).
- Token cache location/format and refresh behavior.
- UX flows: login/status/logout commands (if present).

#### AutoMaker Reference Implementation Analysis

**Objective**: Analyze https://github.com/AutoMaker-Org/automaker to understand its multi-provider authentication architecture.

**Required Analysis**:
1. **Architecture Overview**:
   - How does automaker abstract authentication across multiple providers?
   - What interfaces/traits does it define?
   - How does it handle token storage?

2. **Token Refresh Logic**:
   - How does automaker detect expired tokens?
   - How does it trigger refresh?
   - How does it handle refresh failures?

3. **Provider Integration**:
   - How many providers does automaker support?
   - What patterns emerge for integrating new providers?
   - Are there provider-specific adapters?

4. **CLI UX**:
   - What commands does automaker provide for authentication?
   - How does the user experience the OAuth flow?
   - How are errors communicated?

5. **Security Practices**:
   - Where are tokens stored?
   - How are secrets redacted in logs/errors?
   - What token validation happens?

**Research Strategy**:
1. Clone or web-fetch key files from automaker repo
2. Focus on:
   - Authentication modules/packages
   - CLI command handlers for auth
   - HTTP client/adapter implementations
   - Configuration/credential storage
3. Extract code patterns and architectural decisions
4. Note what works well and what could be improved

**Output Format**:
```markdown
## AutoMaker Authentication Architecture

**Repository**: https://github.com/AutoMaker-Org/automaker

**Key Files Analyzed**:
- [file path]: [purpose]
- [file path]: [purpose]

**Architecture Diagram** (text-based):
```
[ASCII diagram showing component relationships]
```

**Authentication Abstraction**:
[Description of traits/interfaces/classes used]

**Token Lifecycle**:
1. [Step-by-step description of token acquisition]
2. [Token refresh process]
3. [Token revocation]

**Provider Integration Pattern**:
[How new providers are added]

**CLI UX Flow**:
```bash
# Example commands
automaker auth login <provider>
[describe what happens]
```

**Security Practices**:
- Token storage: [location and format]
- Redaction: [how secrets are hidden]
- Validation: [what checks are performed]

**Applicable Patterns for all-in-yum**:
- ✅ Pattern 1: [description]
- ✅ Pattern 2: [description]
- ⚠️ Pattern 3: [description + why it needs adaptation]

**Anti-Patterns to Avoid**:
- ❌ Issue 1: [description]
```

#### grok-cli Authentication Analysis

**Objective**: Analyze https://github.com/airplne/grok-cli to determine whether it uses OAuth/device flow or API keys, how it stores credentials, and how it attaches auth to requests.

**Required Analysis**:
1. **Auth Type**:
   - OAuth access tokens vs API keys/PATs (do not infer; prove from code/docs).
2. **Credential Storage**:
   - Token/key cache location and format
   - Whether refresh tokens are stored and how they are protected
3. **Request Attachment**:
   - Where and how auth is applied to HTTP requests
4. **CLI UX**:
   - login/status/logout commands and expected user interaction

**Output Format**:
```markdown
## grok-cli Authentication

**Repository**: https://github.com/airplne/grok-cli

**Key Files Analyzed**:
- [file path or GitHub permalink]: [purpose]

**Auth Type**: [OAuth access token / API key / UNKNOWN]

**Credential Storage**:
- Location: [path] (or UNKNOWN)
- Format: [json/toml/etc] (or UNKNOWN)
- Refresh behavior: [how refresh works] (or UNKNOWN)

**Request Attachment**:
- [Where token/key is attached; headers/query; cite code]

**Applicable Patterns for all-in-yum**:
- ✅ Pattern 1: [description]
```

### Section 4: Auth Broker/Gateway Architecture Research

**Objective**: Research how CLI tools handle authentication when providers don't support OAuth/IAM directly, or when “no manual API keys” is mandated.

**Required Research**:
1. **Common Broker Patterns**:
   - What are the standard architectures?
   - How do brokers issue short-lived tokens?
   - How do brokers interface with upstream provider APIs?

2. **Token Exchange Flows**:
   - How does a CLI exchange user identity for provider access?
   - What protocols are used? (OAuth token exchange, custom JWT, etc.)

3. **Enterprise Examples**:
   - Find examples of enterprise CLI tools with auth brokers
   - Cloud provider CLIs (AWS, GCP, Azure) patterns
   - Infrastructure tools (Terraform, Kubernetes) patterns

4. **Security Considerations**:
   - How are provider API keys protected in the broker?
   - How is the broker itself secured?
   - Audit and logging requirements

**Research Strategy**:
1. Search for:
   - "CLI authentication broker patterns"
   - "API gateway authentication for CLI tools"
   - "OAuth token exchange for CLI"
   - "AWS CLI authentication architecture"
   - "kubectl authentication flow"
2. Review architecture blog posts and white papers
3. Check for open-source broker implementations

**Output Format**:
```markdown
## Auth Broker/Gateway Patterns

**Pattern 1: [Name]**
- **Description**: [how it works]
- **Architecture**: [diagram or description]
- **Pros**: [benefits]
- **Cons**: [limitations]
- **Examples**: [real-world implementations]
- **Applicability to all-in-yum**: [HIGH/MEDIUM/LOW + reasoning]

[Repeat for 3-5 patterns]

**Recommended Approach**:
[Based on research, which pattern best fits all-in-yum's requirements?]

**Implementation Scope**:
- **In-Repo Changes**: [what changes in all-in-yum]
- **External Infrastructure**: [what needs to be deployed separately]
- **Dependencies**: [what the broker depends on]

**Security Architecture**:
```
[Diagram showing how secrets flow through the system]
```

**Token Format Recommendation**:
[JWT vs opaque token + reasoning]
```

### Section 5: Enterprise SSO Integration Patterns

**Objective**: Research how CLI tools integrate with enterprise identity providers.

**Required Research**:
1. **OIDC for CLIs**:
   - How do CLIs use OpenID Connect?
   - Device authorization grant flow details
   - PKCE flow for CLIs

2. **SAML Integration**:
   - Can CLIs use SAML? How?
   - SAML-to-OAuth bridge patterns

3. **Identity-to-Access Mapping**:
   - How is user identity mapped to provider access?
   - Role-based access control patterns
   - Attribute-based access control patterns

4. **Common Identity Providers**:
   - Okta integration patterns
   - Azure AD integration patterns
   - Auth0 integration patterns
   - Google Workspace integration patterns

**Research Strategy**:
1. Search for:
   - "CLI OIDC device flow"
   - "CLI authentication with Okta"
   - "Azure AD CLI authentication"
   - "Enterprise SSO for command line tools"
2. Review official documentation from IdP vendors
3. Find example integrations in popular CLI tools

**Output Format**:
```markdown
## Enterprise SSO Integration

**OIDC Device Flow**:
- **Description**: [how it works for CLIs]
- **Flow Diagram**: [text-based flow]
- **Pros/Cons**: [analysis]
- **Example Implementation**: [link or code snippet]

**PKCE Flow**:
- **Description**: [how it works for CLIs]
- **Flow Diagram**: [text-based flow]
- **Pros/Cons**: [analysis]
- **Example Implementation**: [link or code snippet]

**SAML Integration**:
- **Feasibility for CLIs**: [assessment]
- **Bridge Patterns**: [if SAML→OAuth bridge needed]

**Identity Provider Integration**:

### Okta
- **CLI Integration Pattern**: [description]
- **Documentation**: [link]
- **Example Commands**: [if available]

### Azure AD
- **CLI Integration Pattern**: [description]
- **Documentation**: [link]
- **Example Commands**: [if available]

[Repeat for Auth0, Google Workspace]

**Recommended Approach for all-in-yum**:
[Which pattern best fits the requirements?]

**Integration Architecture**:
```
[Diagram: User → IdP → all-in-yum CLI → Auth Broker → Providers]
```
```

### Section 6: Token Management Best Practices

**Objective**: Research security best practices for token handling in CLI tools.

**Required Research**:
1. **Secure Storage**:
   - Platform-specific keychains (macOS, Windows, Linux)
   - Encrypted file storage patterns
   - Memory-only storage for short-lived operations

2. **Token Refresh**:
   - Proactive vs reactive refresh strategies
   - Handling refresh failures gracefully
   - Retry logic and backoff

3. **Revocation**:
   - How to revoke tokens locally and remotely
   - Token cleanup on logout
   - Handling revoked tokens gracefully

4. **Redaction and Logging**:
   - What token parts can be logged safely?
   - How to redact tokens in error messages?
   - Audit logging requirements

5. **Expiration Handling**:
   - How to detect expired tokens before use?
   - User communication for expired sessions
   - Grace periods and warnings

**Research Strategy**:
1. Search for:
   - "OAuth token security best practices CLI"
   - "Secure token storage command line"
   - "Token refresh patterns CLI tools"
2. Review OWASP guidelines for OAuth
3. Check security advisories for CLI tools

**Output Format**:
```markdown
## Token Management Best Practices

**Secure Storage**:
- **Platform Keychain**: [when to use, pros/cons]
- **Encrypted File**: [when to use, pros/cons]
- **Best Practice for all-in-yum**: [recommendation]

**Token Refresh Strategy**:
- **Proactive Refresh**: [description, when to use]
- **Reactive Refresh**: [description, when to use]
- **Recommendation**: [which strategy + reasoning]
- **Implementation Pattern**: [pseudo-code or description]

**Revocation**:
- **Local Revocation**: [how to clear tokens]
- **Remote Revocation**: [how to notify provider]
- **Logout Flow**: [step-by-step]

**Redaction Rules**:
- **Access Tokens**: [how to redact in logs/errors]
- **Refresh Tokens**: [how to redact]
- **Example Regex**: `Bearer ey[A-Za-z0-9-_]+` → `Bearer ey***[REDACTED]`

**Expiration Handling**:
- **Pre-emptive Check**: [check expiry before use]
- **User Communication**: [what to tell user]
- **Example UX**:
  ```
  Error: Authentication expired (token valid until 2025-01-12 14:30 UTC)
  Run: aiy auth login anthropic
  ```

**Security Checklist**:
- ✅ Tokens never in stdout/stderr
- ✅ Tokens never in config files
- ✅ Tokens stored encrypted at rest
- ✅ Tokens cleared on logout
- ✅ Token expiry checked before use
- ✅ Refresh failures handled gracefully
- ✅ Network errors don't leak tokens
```

### Section 7: Comparative Analysis & Recommendations

**Objective**: Synthesize all research into actionable recommendations.

**Required Output**:
1. **Provider Readiness Summary**:
   - Which providers support OAuth natively?
   - Which providers require an auth broker?
   - What's the migration path for each?

2. **Architecture Recommendation**:
   - Direct OAuth for providers that support it
   - Auth broker architecture for providers that don't
   - SSO integration points

3. **Implementation Complexity**:
   - Estimate of scope (small/medium/large)
   - Key technical challenges
   - Dependencies on external services

4. **Security Assessment**:
   - Risk analysis of each approach
   - Mitigation strategies
   - Compliance considerations

**Output Format**:
```markdown
## Comparative Analysis

**Provider OAuth Support Summary**:

| Provider | OAuth Native? | Auth Flow | Broker Needed? | Migration Complexity |
|----------|---------------|-----------|----------------|----------------------|
| xAI      | [YES/NO]      | [flow]    | [YES/NO]       | [LOW/MED/HIGH]       |
| Anthropic| [YES/NO]      | [flow]    | [YES/NO]       | [LOW/MED/HIGH]       |
| Google   | [YES/NO]      | [flow]    | [YES/NO]       | [LOW/MED/HIGH]       |
| OpenAI   | [YES/NO]      | [flow]    | [YES/NO]       | [LOW/MED/HIGH]       |

**Recommended Architecture**:

```
┌─────────────────────────────────────────────────────────────┐
│                     all-in-yum CLI                          │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │ auth login   │  │ auth status  │  │ auth logout  │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
│           │                 │                 │             │
│           └─────────────────┴─────────────────┘             │
│                          │                                  │
│                   ┌──────▼──────┐                          │
│                   │ TokenManager│                          │
│                   │  (Storage)  │                          │
│                   └──────┬──────┘                          │
└──────────────────────────┼─────────────────────────────────┘
                           │
        ┌──────────────────┴──────────────────┐
        │                                     │
   ┌────▼─────┐                        ┌─────▼──────┐
   │ Direct   │                        │ Auth Broker│
   │ OAuth    │                        │ (for non-  │
   │(Provider1│                        │ OAuth APIs)│
   │Provider2)│                        └─────┬──────┘
   └────┬─────┘                              │
        │                                    │
   ┌────▼──────────────────┐           ┌────▼────────┐
   │ Provider OAuth APIs   │           │ Provider    │
   │ (Native OAuth)        │           │ API Keys    │
   └───────────────────────┘           │ (Managed)   │
                                       └─────────────┘
```

**Layered Fallback Strategy**:
1. **Tier 1 - Direct OAuth**: Use provider native OAuth where available
2. **Tier 2 - Auth Broker**: Deploy broker for providers without OAuth
3. **Tier 3 - Admin Provisioned**: Fallback for air-gapped/offline scenarios

**Implementation Roadmap**:
- **Phase 1**: [what to build first]
- **Phase 2**: [what to build second]
- **Phase 3**: [what to build third]

**Key Technical Challenges**:
1. [Challenge 1 + mitigation]
2. [Challenge 2 + mitigation]
3. [Challenge 3 + mitigation]

**Security Risk Analysis**:

| Risk | Severity | Mitigation |
|------|----------|------------|
| [Risk 1] | HIGH/MED/LOW | [Strategy] |
| [Risk 2] | HIGH/MED/LOW | [Strategy] |

**Compliance Considerations**:
- SOC2: [requirements and how architecture meets them]
- GDPR: [data handling requirements]
- Enterprise: [audit logging, access control]

**Dependencies**:
- **External Services**: [what needs to be deployed]
- **Provider Support**: [what providers need to enable]
- **Infrastructure**: [what infrastructure is needed]
```

## Research Task Execution Steps

### Step 0: Repo Ground Truth (Priority: CRITICAL)
1. Inspect the all-in-yum adapters and credential manager code locally.
2. Document current auth mechanisms for each adapter with `path:line` citations.
3. Document current credential backends + any unimplemented variants with `path:line` citations.
4. Document config keys relevant to auth (base URLs, provider IDs, credential backend selection) with `path:line` citations.

**Validation**: The report’s “Repo Ground Truth” section contains `path:line` citations for every claim.

### Step 1: Provider OAuth Verification (Priority: CRITICAL)
1. Search for official OAuth documentation for each provider
2. Verify if OAuth is for user accounts only or also for API access
3. Document exact flows, token formats, and integration methods
4. Save findings to Section 1 of the report

**Validation**: Can answer "Does provider X support OAuth for their AI API?" with citations

### Step 2: Cloud IAM Alternatives Research (Priority: CRITICAL)
1. Research cloud-hosted alternatives (Bedrock/Vertex/Azure/OCI as applicable)
2. Confirm model/provider availability with official docs (do not infer)
3. Document IAM auth mechanisms and CLI-friendly credential acquisition paths
4. Save findings to Section 2 of the report

**Validation**: Each cloud alternative has “availability” + “auth mechanism” proven with official docs, or explicitly marked UNKNOWN.

### Step 3: Coding CLI Reference Analysis (Priority: HIGH)
1. Access the automaker repository via web search or direct URL
2. Identify authentication-related code
3. Extract patterns and architectural decisions
4. Document applicable lessons for all-in-yum
5. Repeat for `grok-cli` (auth mechanism, token cache, UX flows)
6. Save findings to Section 3 of the report

**Validation**: Can describe both tools’ auth architecture and extract 3+ applicable patterns with citations.

### Step 4: Auth Broker Research (Priority: HIGH)
1. Search for enterprise CLI authentication patterns
2. Identify 3-5 common broker architectures
3. Analyze pros/cons of each
4. Recommend approach for all-in-yum
5. Save findings to Section 4 of the report

**Validation**: Can propose a concrete auth broker design with justification

### Step 5: SSO Integration Research (Priority: MEDIUM)
1. Research OIDC device flow and PKCE flow
2. Document integration patterns with major IdPs
3. Identify best practices for CLI SSO
4. Save findings to Section 5 of the report

**Validation**: Can describe how a user would authenticate via enterprise SSO

### Step 6: Token Management Research (Priority: MEDIUM)
1. Research secure token storage methods
2. Document refresh and revocation patterns
3. Identify redaction and logging best practices
4. Save findings to Section 6 of the report

**Validation**: Can provide security checklist and implementation patterns

### Step 7: Synthesis & Recommendations (Priority: HIGH)
1. Combine all research findings
2. Create provider readiness matrix
3. Propose architecture recommendation
4. Document implementation roadmap
5. Save findings to Section 7 of the report

**Validation**: Report provides clear go/no-go decision for each provider and a concrete implementation plan

## Validation Commands

After completing research, verify the report is complete:

```bash
# Check report exists
ls docs/oauth-research-findings.md

# Verify all sections present
grep "^## " docs/oauth-research-findings.md

# Expected sections:
# - Repo Ground Truth
# - Provider: xAI (Grok)
# - Provider: Anthropic (Claude)
# - Provider: Google (Gemini)
# - Provider: OpenAI (Codex)
# - Cloud IAM / Enterprise Alternatives Matrix
# - AutoMaker Authentication Architecture
# - grok-cli Authentication
# - Auth Broker/Gateway Patterns
# - Enterprise SSO Integration
# - Token Management Best Practices
# - Comparative Analysis
```

## Acceptance Criteria

1. ✅ **Repo Ground Truth Complete**: All auth/credential/config claims about all-in-yum have `path:line` citations (Section 0).
2. ✅ **Direct Provider Matrix Complete**: All 4 providers researched with OAuth support status verified (Section 1).
3. ✅ **Cloud IAM Matrix Complete**: Bedrock/Vertex/Azure/OCI evaluated with explicit “availability” and “auth mechanism” proven with official docs or marked UNKNOWN (Section 2).
4. ✅ **Citations Provided**: Every provider capability claim backed by official documentation links; every repo claim backed by `path:line`.
5. ✅ **Coding CLI References Analyzed**: AutoMaker and grok-cli auth patterns documented with citations (Section 3).
6. ✅ **Broker Architecture Proposed**: Concrete design for providers without direct OAuth/IAM, with scope and tradeoffs (Section 4).
7. ✅ **SSO Integration Documented**: Clear path for enterprise IdP integration (Section 5).
8. ✅ **Security Best Practices**: Token storage, refresh, revocation, and redaction patterns documented (Section 6).
9. ✅ **Actionable Recommendations**: Clear go/no-go per provider + phased implementation roadmap (Section 7).

## Risk Notes

### Research Risks
- **Risk**: Provider documentation may be unclear or contradictory
  - **Mitigation**: Use multiple sources; flag ambiguities for clarification
- **Risk**: AutoMaker repo may use different language/framework
  - **Mitigation**: Focus on architectural patterns, not specific code
- **Risk**: OAuth may not exist for some providers despite claims
  - **Mitigation**: Verify with official docs; propose broker as fallback

### Scope Risks
- **Risk**: Research could expand indefinitely
  - **Mitigation**: Time-box each section; aim for 80% confidence not 100% perfection
- **Risk**: Over-engineering the solution before understanding constraints
  - **Mitigation**: Research team gathers facts; implementation team decides

## Output Deliverable

**Single File**: `/home/aip0rt/Desktop/all-in-yum/docs/oauth-research-findings.md`

This report will be used by the implementation planning team to write the actual OAuth/IAM Migration Implementation PRP.

## Success Criteria

The research is complete when:
1. All required sections of the report are written (Sections 0–7)
2. Every provider has OAuth support status determined for direct APIs (with official citations)
3. Cloud IAM alternatives are evaluated (with official citations or explicit UNKNOWN)
4. At least 3 applicable patterns extracted from AutoMaker and/or grok-cli
5. Auth broker architecture is proposed (if needed)
6. SSO integration path is documented
7. Security best practices are enumerated
8. Final recommendations synthesize all findings into actionable next steps

The report should enable the implementation team to make informed architectural decisions without needing to do additional research.
