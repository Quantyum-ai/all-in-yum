# PRP: OAuth/IAM Authentication Migration for all-in-yum

## Title
Eliminate Manual API Keys via OAuth, Cloud IAM, and Enterprise SSO Integration

## Objective
Migrate all-in-yum from user-entered API keys to OAuth-based authentication, cloud IAM integration, and enterprise SSO support. Users will authenticate via device code flows (SSO-backed) and obtain time-limited credentials. Where providers lack OAuth, leverage cloud platforms (AWS Bedrock, Azure OpenAI, GCP Vertex AI) or support auth broker integration for centralized key management.

## Scope

### In Scope
1. **New CLI Auth Commands**: `aiy auth login/status/logout` with provider-specific flows
2. **TokenManager**: Secure storage for OAuth tokens (access, refresh, metadata) with automatic refresh
3. **Provider Strategy (Layered)**:
   - **Tier 1 (Direct OAuth)**: Google Gemini via ADC/OAuth
   - **Tier 2 (Cloud IAM)**: Anthropic via Bedrock/Vertex, OpenAI via Azure
   - **Tier 3 (Auth Broker)**: xAI via enterprise broker, fallback for others
4. **Adapter Refactoring**: Update HTTP clients to use tokens instead of API keys
5. **Config Extension**: Per-provider auth mode configuration (direct/azure/bedrock/vertex/broker)
6. **Security Hardening**: Token redaction, expiry checks, secure storage, no secrets in logs
7. **Offline Testing**: Mock token flows without network, preserve `--offline` capability
8. **Documentation**: Migration guide, runbooks, enterprise setup instructions

### Non-Goals
- Implementing a full auth broker server (provide integration points and reference docs)
- Modifying core adapter logic beyond auth injection
- Breaking existing API key workflows during transition period
- Supporting SAML directly (use OIDC/OAuth with IdP bridges)

---

## Provider Capability Matrix (From Research)

Based on GPT-5 Pro research findings:

| Provider | Native OAuth? | Best Auth Flow | Cloud IAM Alternative | Broker Needed? | Priority |
|----------|---------------|----------------|-----------------------|----------------|----------|
| **Google (Gemini)** | ✅ YES | OAuth2 ADC/Device Code | N/A (native) | ❌ NO | **HIGH** |
| **OpenAI (Codex)** | ❌ NO | API Key | ✅ Azure OpenAI (AAD) | ⚠️ YES (if no Azure) | **HIGH** |
| **Anthropic (Claude)** | ❌ NO | API Key | ✅ AWS Bedrock, GCP Vertex | ⚠️ YES (if no cloud) | **MEDIUM** |
| **xAI (Grok)** | ❌ NO | API Key | ❌ NONE | ✅ YES (required) | **MEDIUM** |

**Key Findings**:
- Only Google supports native OAuth for API access
- OpenAI and Anthropic available via cloud platforms with enterprise IAM
- xAI requires auth broker or continued API key usage (no alternatives found)

---

## Step 0: Verify Current Authentication (Repo Ground Truth)

Before designing new flows, verify existing auth implementation in the repo:

### Current Auth per Provider (Verify These Paths)

**File**: `crates/aiy-core/src/security/credential_manager.rs`
- **CredentialManager** stores API keys in three backends:
  - `EncryptedFile`: AES-256-GCM encrypted store on disk (path chosen by the CLI; see `crates/aiy-cli/src/credential_helper.rs`)
  - `SystemKeychain`: OS-native storage via `keyring` crate
  - `SecretManager`: Cloud secret managers (**NOT IMPLEMENTED**; currently `todo!()`)
- **Storage flow**:
  - `unlock(password)` derives the master key via Argon2id and loads the encrypted store
  - `store_key(provider, key)` encrypts and persists
  - `get_key(provider)` retrieves decrypted key
  - `delete_key(provider)` removes a stored credential

**File**: `crates/aiy-cli/src/credential_helper.rs`
- Defines the on-disk credential file path used by the CLI: `dirs::config_dir()/all-in-yum/credentials.enc`
- Handles non-interactive unlock via `AIY_CREDENTIALS_PASSWORD`

**File**: `crates/aiy-cli/src/commands/credentials.rs`
- CLI prompts for API keys (expected, but verify actual prompt functions exist)
- Backend selection likely via config or defaults to `EncryptedFile`

### Adapter Auth Injection (Verify These Patterns)

Current repo behavior (verify in code before changing):
- **Grok (xAI)**: `Authorization: Bearer <key>` in `crates/aiy-adapter-grok/src/client.rs`
- **Codex (OpenAI)**: `Authorization: Bearer <key>` in `crates/aiy-adapter-codex/src/client.rs`
- **Claude (Anthropic)**: `x-api-key: <key>` in `crates/aiy-adapter-claude/src/client.rs`
- **Gemini (Google AI Studio / Generative Language API)**: `?key=<api_key>` query param in `crates/aiy-adapter-gemini/src/client.rs`

**Verification Command**:
```bash
# Find where API keys are currently attached to requests
rg "Authorization.*Bearer" crates/aiy-adapter-*/src/
rg "x-api-key" crates/aiy-adapter-*/src/
rg "\\?key=" crates/aiy-adapter-*/src/
```

**Expected Current State**:
- No OAuth flows implemented
- All providers use `CredentialManager::get_key("provider")` to retrieve API keys
- Keys stored encrypted or in OS keychain
- No token refresh, no expiry handling, no SSO integration

**ACTION**: Dev team must verify these paths exist before proceeding. If research conflicts with actual code, **repo wins**.

---

## Architecture & Interfaces

### Token Storage Format

**New Struct**: `crates/aiy-core/src/security/token.rs`

```rust
/// OAuth token with metadata for storage and refresh
#[derive(Debug, Clone, Serialize, Deserialize, Zeroize, ZeroizeOnDrop)]
pub struct OAuthToken {
    /// Access token (short-lived, used for API calls)
    #[zeroize(skip)]
    pub access_token: String,

    /// Refresh token (long-lived, used to obtain new access tokens)
    #[zeroize(skip)]
    pub refresh_token: Option<String>,

    /// Token type (usually "Bearer")
    pub token_type: String,

    /// Expiration timestamp (Unix epoch seconds)
    pub expires_at: i64,

    /// OAuth scopes granted
    pub scopes: Vec<String>,

    /// Issuer metadata (IdP endpoint, tenant ID, etc.)
    pub issuer: Option<String>,

    /// Provider-specific metadata (e.g., Azure resource ID, GCP project)
    pub metadata: HashMap<String, String>,
}

impl OAuthToken {
    /// Check if token is expired or expiring soon (within threshold)
    pub fn is_expired(&self, threshold_seconds: i64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        self.expires_at <= now + threshold_seconds
    }

    /// Check if token can be refreshed
    pub fn can_refresh(&self) -> bool {
        self.refresh_token.is_some()
    }
}
```

### Credential Abstraction

**Update**: `crates/aiy-core/src/security/credential.rs` (NEW FILE)

```rust
/// Unified credential type supporting both API keys and OAuth tokens
#[derive(Debug, Clone, Zeroize, ZeroizeOnDrop)]
pub enum Credential {
    /// Legacy API key (static, no expiry)
    ApiKey(String),

    /// OAuth token (time-limited, refreshable)
    OAuth(OAuthToken),

    /// Cloud IAM credential (e.g., AWS SigV4, Azure AAD)
    CloudIam {
        provider: CloudProvider,
        token: OAuthToken,
    },
}

impl Credential {
    /// Get the value to attach to API requests
    pub async fn get_auth_value(&self) -> anyhow::Result<String> {
        match self {
            Credential::ApiKey(key) => Ok(key.clone()),
            Credential::OAuth(token) => {
                if token.is_expired(60) {
                    anyhow::bail!("Token expired, refresh needed");
                }
                Ok(format!("Bearer {}", token.access_token))
            }
            Credential::CloudIam { token, .. } => {
                if token.is_expired(60) {
                    anyhow::bail!("Cloud token expired, refresh needed");
                }
                Ok(format!("Bearer {}", token.access_token))
            }
        }
    }

    /// Check if credential needs refresh
    pub fn needs_refresh(&self) -> bool {
        match self {
            Credential::ApiKey(_) => false,
            Credential::OAuth(token) | Credential::CloudIam { token, .. } => {
                token.is_expired(300) // Refresh if < 5 min remaining
            }
        }
    }
}
```

### TokenManager

**Update**: `crates/aiy-core/src/security/credential_manager.rs`

Add OAuth-specific methods:

```rust
impl CredentialManager {
    /// Store OAuth token for provider
    pub fn store_token(
        &mut self,
        provider: &str,
        token: OAuthToken,
    ) -> Result<(), CredentialError> {
        let serialized = serde_json::to_string(&token)?;
        // Store as if it's a key, but with special prefix
        self.store_key(&format!("oauth:{}", provider), &serialized)
    }

    /// Retrieve OAuth token for provider
    pub fn get_token(&self, provider: &str) -> Result<OAuthToken, CredentialError> {
        let serialized = self.get_key(&format!("oauth:{}", provider))?;
        serde_json::from_str(&serialized)
            .map_err(|e| CredentialError::Other(format!("Invalid token: {}", e)))
    }

    /// Refresh an OAuth token using refresh token
    pub async fn refresh_token(
        &mut self,
        provider: &str,
        refresh_endpoint: &str,
        client_id: &str,
    ) -> Result<OAuthToken, CredentialError> {
        let current = self.get_token(provider)?;
        if !current.can_refresh() {
            return Err(CredentialError::NotFound);
        }

        // Call OAuth refresh endpoint (implement using reqwest)
        let new_token = oauth::refresh_token(
            refresh_endpoint,
            client_id,
            current.refresh_token.as_ref().unwrap(),
        ).await?;

        self.store_token(provider, new_token.clone())?;
        Ok(new_token)
    }
}
```

### Provider Auth Strategies

**New Module**: `crates/aiy-core/src/auth/strategy.rs`

```rust
/// Authentication strategy per provider
#[async_trait]
pub trait AuthStrategy: Send + Sync {
    /// Initiate login flow (returns instructions for user)
    async fn login(&self) -> anyhow::Result<LoginFlow>;

    /// Poll for completed authentication
    async fn poll_completion(&self, flow: &LoginFlow) -> anyhow::Result<Option<Credential>>;

    /// Refresh credential if expired
    async fn refresh(&self, credential: &Credential) -> anyhow::Result<Credential>;

    /// Logout (revoke tokens if possible)
    async fn logout(&self, credential: &Credential) -> anyhow::Result<()>;
}

pub enum LoginFlow {
    /// Device code flow (show user code and URL)
    DeviceCode {
        device_code: String,
        user_code: String,
        verification_url: String,
        expires_in: u64,
        interval: u64,
    },

    /// Browser-based PKCE flow (open browser, wait for callback)
    BrowserPKCE {
        auth_url: String,
        state: String,
        code_verifier: String,
    },

    /// Manual API key entry (legacy fallback)
    ManualKey {
        instructions: String,
    },
}
```

**Implement strategies**:
- `GoogleOAuthStrategy`: Device code flow to Google OAuth
- `AzureAdStrategy`: Device code flow to Azure AD for OpenAI
- `AwsBedrockStrategy`: AWS SSO or assume role for Bedrock
- `VertexAiStrategy`: Google OAuth for Vertex AI (Anthropic/Gemini)
- `AuthBrokerStrategy`: Device code flow to enterprise broker
- `ApiKeyStrategy`: Legacy manual key entry (deprecation path)

---

## Implementation Plan

### Task 1: Create OAuth Token Infrastructure (Parallel)

**Files**:
- `crates/aiy-core/src/security/token.rs` (NEW)
- `crates/aiy-core/src/security/credential.rs` (NEW)
- `crates/aiy-core/src/auth/strategy.rs` (NEW)
- `crates/aiy-core/src/auth/oauth_client.rs` (NEW) - HTTP client for OAuth endpoints

**Subtasks**:
1. Define `OAuthToken` struct with serialization, zeroization
2. Implement `Credential` enum and `get_auth_value()` method
3. Create `AuthStrategy` trait with device code and PKCE patterns
4. Implement generic OAuth HTTP client (device authorize, token exchange, refresh) behind a trait so tests can use an in-process mock transport (no TCP binding, no network)

**Validation**:
```bash
cargo check -p aiy-core
cargo test -p aiy-core --lib oauth
```

**Acceptance**:
- `OAuthToken` can be serialized/deserialized
- `Credential` correctly detects expiry and formats auth values
- Unit tests for expiry detection pass

---

### Task 2: Extend CredentialManager for OAuth (Parallel)

**File**: `crates/aiy-core/src/security/credential_manager.rs`

**Subtasks**:
1. Add `store_token()` and `get_token()` methods (serialize OAuth to encrypted storage)
2. Add `refresh_token()` method that calls OAuth refresh endpoint
3. Update `lock()` and `unlock()` to handle both API keys and tokens
4. Ensure tokens are zeroized from memory after use

**Validation**:
```bash
cargo test -p aiy-core credential_manager
```

**Acceptance**:
- Tokens stored via `store_token()` can be retrieved via `get_token()`
- Refresh flow works (with mock HTTP responses in tests)
- Tokens never appear in debug output (sanitization tests pass)

---

### Task 3: Implement Google OAuth Strategy (HIGH PRIORITY)

**Files**:
- `crates/aiy-core/src/auth/strategies/google.rs` (NEW)

**Implementation**:
```rust
pub struct GoogleOAuthStrategy {
    client_id: String,
    scopes: Vec<String>,
}

#[async_trait]
impl AuthStrategy for GoogleOAuthStrategy {
    async fn login(&self) -> anyhow::Result<LoginFlow> {
        // Call Google device authorization endpoint
        let resp = reqwest::post("https://oauth2.googleapis.com/device/code")
            .form(&[
                ("client_id", &self.client_id),
                ("scope", &self.scopes.join(" ")),
            ])
            .send()
            .await?;

        let data: DeviceCodeResponse = resp.json().await?;

        Ok(LoginFlow::DeviceCode {
            device_code: data.device_code,
            user_code: data.user_code,
            verification_url: data.verification_url,
            expires_in: data.expires_in,
            interval: data.interval,
        })
    }

    async fn poll_completion(&self, flow: &LoginFlow) -> anyhow::Result<Option<Credential>> {
        match flow {
            LoginFlow::DeviceCode { device_code, .. } => {
                // Poll Google token endpoint
                let resp = reqwest::post("https://oauth2.googleapis.com/token")
                    .form(&[
                        ("client_id", &self.client_id),
                        ("device_code", device_code),
                        ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                    ])
                    .send()
                    .await?;

                if resp.status() == 200 {
                    let token: TokenResponse = resp.json().await?;
                    Ok(Some(Credential::OAuth(token.into())))
                } else {
                    Ok(None) // Still pending
                }
            }
            _ => anyhow::bail!("Unsupported flow"),
        }
    }

    async fn refresh(&self, credential: &Credential) -> anyhow::Result<Credential> {
        // Implement refresh token flow for Google
        // ...
    }
}
```

**Decision Point**: **Do we leverage `gcloud` CLI or implement native OAuth?**

**Recommendation**: **Implement native OAuth in Rust**

**Rationale**:
- **Portability**: No external dependency on `gcloud` installation
- **Security**: Full control over token handling, no subprocess risks
- **User Experience**: Unified auth flow across all providers
- **CI/Automation**: Easier to seed tokens programmatically for testing

**Tradeoff**: Slightly more implementation work upfront, but better long-term maintainability.

**Alternative**: If user already has `gcloud` installed and authenticated, detect ADC tokens at `~/.config/gcloud/application_default_credentials.json` and use them (read-only, no shelling out). Provide this as a **convenience fallback** but NOT the primary path.

**Validation**:
```bash
# Unit tests with a mock OAuth transport (no TCP binding)
cargo test -p aiy-core google_oauth --offline

# Integration test (requires real Google OAuth client ID, gated behind feature)
cargo test -p aiy-core google_oauth_integration --features integration-tests
```

**Acceptance**:
- Device code flow generates valid user code and URL
- Polling correctly waits for user authentication
- Refresh token flow works
- Tokens stored securely and retrieved correctly

---

### Task 4: Implement Azure AD Strategy (HIGH PRIORITY)

**Files**:
- `crates/aiy-core/src/auth/strategies/azure.rs` (NEW)

**Implementation**:
Similar to Google, but for Azure AD:
- Device code endpoint: `https://login.microsoftonline.com/{tenant}/oauth2/v2.0/devicecode`
- Token endpoint: `https://login.microsoftonline.com/{tenant}/oauth2/v2.0/token`
- Scopes: `https://cognitiveservices.azure.com/.default` for Azure OpenAI

**Configuration**:
- Tenant ID (from config or env `AZURE_TENANT_ID`)
- Client ID (registered Azure AD app for all-in-yum)
- Resource endpoint (Azure OpenAI resource URL from config)

**Validation**:
```bash
cargo test -p aiy-core azure_ad --offline
```

**Acceptance**:
- Device code flow works for Azure AD
- Tokens scoped correctly for Azure OpenAI
- Refresh works (AAD tokens typically 1 hour lifetime)

---

### Task 5: Implement AWS Bedrock Strategy (MEDIUM PRIORITY)

**Files**:
- `crates/aiy-core/src/auth/strategies/bedrock.rs` (NEW)

**Implementation**:
```rust
pub struct BedrockStrategy {
    region: String,
    profile: Option<String>,
}

#[async_trait]
impl AuthStrategy for BedrockStrategy {
    async fn login(&self) -> anyhow::Result<LoginFlow> {
        // Check if AWS credentials already present
        if let Some(creds) = load_aws_credentials(self.profile.as_deref()).await? {
            // Already authenticated, return immediate success
            return Ok(LoginFlow::Complete(Credential::CloudIam {
                provider: CloudProvider::AwsBedrock,
                token: creds,
            }));
        }

        // Otherwise, guide user to run `aws sso login` or `aws configure`
        Ok(LoginFlow::ManualKey {
            instructions: "Please authenticate with AWS CLI:\n\
                           - For SSO: aws sso login --profile <profile>\n\
                           - For keys: aws configure\n\
                           Then retry this command.".to_string(),
        })
    }

    async fn poll_completion(&self, _flow: &LoginFlow) -> anyhow::Result<Option<Credential>> {
        // Re-check credentials
        if let Some(creds) = load_aws_credentials(self.profile.as_deref()).await? {
            Ok(Some(Credential::CloudIam {
                provider: CloudProvider::AwsBedrock,
                token: creds,
            }))
        } else {
            Ok(None)
        }
    }
}

async fn load_aws_credentials(profile: Option<&str>) -> anyhow::Result<Option<OAuthToken>> {
    // Use AWS SDK to load credentials from:
    // - Environment variables (AWS_ACCESS_KEY_ID, AWS_SECRET_ACCESS_KEY)
    // - AWS SSO cache (~/.aws/sso/cache/*.json)
    // - ~/.aws/credentials file
    //
    // Convert AWS credentials to OAuthToken format (for consistency)
    // Note: AWS credentials don't have "refresh token" in OAuth sense,
    // but STS temporary creds do have expiry

    use aws_config::meta::region::RegionProviderChain;
    use aws_config::profile::ProfileFileCredentialsProvider;

    let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
    let config = aws_config::from_env()
        .region(region_provider)
        .profile_name(profile.unwrap_or("default"))
        .load()
        .await;

    // Check if credentials present
    // ...
}
```

**Decision Point**: **Do we shell out to `aws` CLI or use AWS SDK?**

**Recommendation**: **Use AWS SDK crate (`aws-config`, `aws-sdk-bedrockruntime`)**

**Rationale**:
- Native Rust integration, no subprocess
- Can detect credentials from all standard AWS sources
- Better error handling and security
- Enables programmatic credential refresh via STS

**Tradeoff**: Adds AWS SDK dependency (~5 crates), but worth it for robustness.

**Validation**:
```bash
# Mock AWS credentials in test env
AWS_ACCESS_KEY_ID=test AWS_SECRET_ACCESS_KEY=test cargo test -p aiy-core bedrock --offline
```

**Acceptance**:
- Detects existing AWS credentials (env, SSO cache, file)
- Falls back to helpful error message if not authenticated
- Credentials correctly formatted for Bedrock SigV4 signing

---

### Task 6: Implement Auth Broker Strategy (MEDIUM PRIORITY)

**Files**:
- `crates/aiy-core/src/auth/strategies/broker.rs` (NEW)

**Implementation**:
```rust
pub struct AuthBrokerStrategy {
    broker_url: String,
    provider: String, // e.g., "openai", "xai"
    client_id: Option<String>,
}

#[async_trait]
impl AuthStrategy for AuthBrokerStrategy {
    async fn login(&self) -> anyhow::Result<LoginFlow> {
        // Call broker's /device endpoint
        let resp = reqwest::post(format!("{}/auth/device", self.broker_url))
            .json(&json!({
                "provider": self.provider,
                "client_id": self.client_id,
            }))
            .send()
            .await?;

        let data: DeviceCodeResponse = resp.json().await?;

        Ok(LoginFlow::DeviceCode {
            device_code: data.device_code,
            user_code: data.user_code,
            verification_url: data.verification_url,
            expires_in: data.expires_in,
            interval: data.interval,
        })
    }

    async fn poll_completion(&self, flow: &LoginFlow) -> anyhow::Result<Option<Credential>> {
        match flow {
            LoginFlow::DeviceCode { device_code, .. } => {
                // Poll broker's /token endpoint
                let resp = reqwest::post(format!("{}/auth/token", self.broker_url))
                    .json(&json!({
                        "device_code": device_code,
                        "provider": self.provider,
                    }))
                    .send()
                    .await?;

                if resp.status() == 200 {
                    let token: TokenResponse = resp.json().await?;
                    Ok(Some(Credential::OAuth(token.into())))
                } else if resp.status() == 428 {
                    Ok(None) // Pending
                } else {
                    anyhow::bail!("Auth failed: {}", resp.status())
                }
            }
            _ => anyhow::bail!("Unsupported flow"),
        }
    }
}
```

**Note**: This strategy assumes an enterprise-deployed auth broker. We do NOT implement the broker server itself, but provide:
1. Clear API contract (OpenAPI spec in docs)
2. Reference implementation guide (using Kong/Apigee/custom)
3. CLI integration points

**Validation**:
```bash
# Mock broker server in tests
cargo test -p aiy-core auth_broker --offline
```

**Acceptance**:
- CLI can initiate device flow with broker
- CLI correctly polls for completion
- Broker-issued tokens stored and refreshed
- Clear error messages if broker unavailable

---

### Task 7: Add CLI Auth Commands (Sequential after Tasks 1-6)

**File**: `crates/aiy-cli/src/commands/auth.rs` (NEW)

**Commands**:
```rust
#[derive(Parser)]
pub enum AuthCommand {
    /// Login to a provider
    Login {
        /// Provider name (openai, anthropic, google, xai)
        provider: String,

        /// Auth method (direct, azure, bedrock, vertex, broker)
        #[arg(long)]
        via: Option<String>,

        /// Force browser-based flow instead of device code
        #[arg(long)]
        browser: bool,
    },

    /// Show authentication status
    Status {
        /// Show details for specific provider (default: all)
        provider: Option<String>,
    },

    /// Logout from a provider
    Logout {
        /// Provider name (or "all" for all providers)
        provider: String,
    },
}

impl AuthCommand {
    pub async fn run(&self) -> anyhow::Result<()> {
        match self {
            AuthCommand::Login { provider, via, browser } => {
                handle_login(provider, via.as_deref(), *browser).await
            }
            AuthCommand::Status { provider } => {
                handle_status(provider.as_deref()).await
            }
            AuthCommand::Logout { provider } => {
                handle_logout(provider).await
            }
        }
    }
}

async fn handle_login(provider: &str, via: Option<&str>, browser: bool) -> anyhow::Result<()> {
    let config_path = PipelineConfig::config_path()?;
    let config = PipelineConfig::load(&config_path).unwrap_or_else(|_| PipelineConfig::default());

    // NOTE: `aiy auth` must work even when `credentials.enc` does not exist yet.
    // Do NOT reuse any helper that fails if the encrypted store is missing.
    let (mut cred_manager, needs_unlock) = credential_helper::create_credential_manager(&config)?;
    if needs_unlock {
        // For auth flows, allow initializing a new encrypted store.
        // Prefer `AIY_CREDENTIALS_PASSWORD`; otherwise prompt if interactive.
        let password = std::env::var(credential_helper::CREDENTIALS_PASSWORD_ENV)
            .ok()
            .or_else(|| {
                if std::io::stdin().is_terminal() {
                    rpassword::prompt_password("Enter credentials password: ").ok()
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Credentials backend is locked. Set {} or run interactively.",
                    credential_helper::CREDENTIALS_PASSWORD_ENV
                )
            })?;

        cred_manager.unlock(&password)?;
    }

    // Select strategy based on provider and via
    let strategy: Box<dyn AuthStrategy> = match (provider, via) {
        ("google", None) | ("gemini", None) => {
            Box::new(GoogleOAuthStrategy::new(&config)?)
        }
        ("openai", Some("azure")) | ("codex", Some("azure")) => {
            Box::new(AzureAdStrategy::new(&config)?)
        }
        ("anthropic", Some("bedrock")) | ("claude", Some("bedrock")) => {
            Box::new(BedrockStrategy::new(&config)?)
        }
        ("anthropic", Some("vertex")) | ("claude", Some("vertex")) => {
            Box::new(VertexAiStrategy::new(&config)?)
        }
        ("xai", _) | ("grok", _) => {
            // Check if broker configured
            if let Some(broker_url) = config.auth_broker_url.as_ref() {
                Box::new(AuthBrokerStrategy::new(broker_url, provider)?)
            } else {
                anyhow::bail!(
                    "xAI requires an auth broker. Configure `auth_broker_url` in config\n\
                     or use legacy API key mode with `aiy credentials set xai`"
                );
            }
        }
        _ => {
            anyhow::bail!(
                "Unknown provider or auth method. Supported:\n\
                 - google (direct OAuth)\n\
                 - openai --via azure\n\
                 - anthropic --via bedrock|vertex\n\
                 - xai (requires broker)"
            );
        }
    };

    // Initiate login flow
    println!("🔐 Starting authentication for {}...", provider);
    let flow = strategy.login().await?;

    match flow {
        LoginFlow::DeviceCode { user_code, verification_url, expires_in, interval, device_code } => {
            println!("\n📱 Please visit:\n    {}", verification_url);
            println!("\n🔑 Enter code: {}\n", user_code);
            println!("⏳ Expires in {} seconds\n", expires_in);

            // Poll for completion
            let start = Instant::now();
            loop {
                if start.elapsed().as_secs() > expires_in {
                    anyhow::bail!("Authentication timeout");
                }

                tokio::time::sleep(Duration::from_secs(interval)).await;

                if let Some(credential) = strategy.poll_completion(&flow).await? {
                    // Store credential
                    match credential {
                        Credential::OAuth(token) => {
                            cred_manager.store_token(provider, token)?;
                        }
                        Credential::CloudIam { token, .. } => {
                            cred_manager.store_token(provider, token)?;
                        }
                        _ => {}
                    }

                    println!("✅ Authentication successful!");
                    println!("   Credentials stored securely.");
                    break;
                }
            }
        }
        LoginFlow::ManualKey { instructions } => {
            println!("{}", instructions);
        }
        _ => {
            anyhow::bail!("Unsupported flow type");
        }
    }

    Ok(())
}

async fn handle_status(provider: Option<&str>) -> anyhow::Result<()> {
    let config_path = PipelineConfig::config_path()?;
    let config = PipelineConfig::load(&config_path).unwrap_or_else(|_| PipelineConfig::default());

    let (mut cred_manager, needs_unlock) = credential_helper::create_credential_manager(&config)?;
    let is_unlocked = if needs_unlock {
        if let Ok(password) = std::env::var(credential_helper::CREDENTIALS_PASSWORD_ENV) {
            cred_manager.unlock(&password)?;
            true
        } else {
            false
        }
    } else {
        true
    };

    let providers = if let Some(p) = provider {
        vec![p.to_string()]
    } else {
        vec!["google".to_string(), "openai".to_string(), "anthropic".to_string(), "xai".to_string()]
    };

    println!("🔐 Authentication Status:\n");

    for p in providers {
        if !is_unlocked {
            println!("  {} 🔒 LOCKED", p.to_uppercase());
            println!(
                "     Set {} (or run interactively) to show details",
                credential_helper::CREDENTIALS_PASSWORD_ENV
            );
            println!();
            continue;
        }

        // Try to load token (new method added in Task 2)
        match cred_manager.get_token(&p) {
            Ok(token) => {
                let status = if token.is_expired(0) {
                    "❌ EXPIRED"
                } else if token.is_expired(300) {
                    "⚠️  EXPIRING SOON"
                } else {
                    "✅ VALID"
                };

                let expires = DateTime::<Utc>::from_timestamp(token.expires_at, 0)
                    .map(|dt| dt.to_rfc3339())
                    .unwrap_or_else(|| "unknown".to_string());

                println!("  {} {}", p.to_uppercase(), status);
                println!("     Type: OAuth");
                println!("     Expires: {}", expires);
                println!("     Refresh: {}", if token.can_refresh() { "✅" } else { "❌" });
            }
            Err(_) => {
                // Try API key
                match cred_manager.get_key(&p) {
                    Ok(_) => {
                        println!("  {} ✅ VALID (API Key)", p.to_uppercase());
                        println!("     Type: API Key (legacy)");
                    }
                    Err(_) => {
                        println!("  {} ❌ NOT CONFIGURED", p.to_uppercase());
                    }
                }
            }
        }
        println!();
    }

    Ok(())
}

async fn handle_logout(provider: &str) -> anyhow::Result<()> {
    let config_path = PipelineConfig::config_path()?;
    let config = PipelineConfig::load(&config_path).unwrap_or_else(|_| PipelineConfig::default());

    let (mut cred_manager, needs_unlock) = credential_helper::create_credential_manager(&config)?;
    if needs_unlock {
        let password = std::env::var(credential_helper::CREDENTIALS_PASSWORD_ENV)
            .ok()
            .or_else(|| {
                if std::io::stdin().is_terminal() {
                    rpassword::prompt_password("Enter credentials password: ").ok()
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Credentials backend is locked. Set {} or run interactively.",
                    credential_helper::CREDENTIALS_PASSWORD_ENV
                )
            })?;
        cred_manager.unlock(&password)?;
    }

    if provider == "all" {
        println!("🔓 Logging out from all providers...");
        // Remove all stored credentials
        for p in ["google", "openai", "anthropic", "xai"] {
            let _ = cred_manager.delete_key(p);
            let _ = cred_manager.delete_key(&format!("oauth:{}", p));
        }
        println!("✅ All credentials cleared.");
    } else {
        println!("🔓 Logging out from {}...", provider);

        // Try to revoke token if OAuth
        if let Ok(token) = cred_manager.get_token(provider) {
            // Attempt revocation (best effort)
            // ...revoke logic...
        }

        // Remove from storage
        cred_manager.delete_key(&format!("oauth:{}", provider))?;
        cred_manager.delete_key(provider)?;

        println!("✅ Credentials cleared for {}.", provider);
    }

    Ok(())
}
```

**Validation**:
```bash
cargo build -p aiy-cli
cargo test -p aiy-cli --offline
```

**Acceptance**:
- `aiy auth login google` initiates device code flow
- `aiy auth status` shows OAuth token status with expiry
- `aiy auth logout google` clears credentials

---

### Task 8: Update Adapters to Use Credentials (Parallel)

**Files**:
- `crates/aiy-adapter-gemini/src/client.rs` (UPDATE)
- `crates/aiy-adapter-grok/src/client.rs` (UPDATE)
- `crates/aiy-adapter-claude/src/client.rs` (UPDATE)
- `crates/aiy-adapter-codex/src/client.rs` (UPDATE)

**Pattern** (example for OpenAI):

```rust
// OLD (API key):
let api_key = credential_manager.lock().await.get_key("openai")?;
request = request.header("Authorization", format!("Bearer {}", api_key));

// NEW (Credential abstraction):
let credential = credential_manager.lock().await.get_credential("openai").await?;

// Check if refresh needed
if credential.needs_refresh() {
    credential = credential_manager.lock().await.refresh_credential("openai").await?;
}

let auth_value = credential.get_auth_value().await?;
request = request.header("Authorization", auth_value);
```

**Subtasks** per adapter:
1. Replace `get_key()` calls with `get_credential()`
2. Add refresh check before each request
3. Handle credential expiry gracefully (retry once after refresh)
4. Update error messages to guide re-login

**Validation**:
```bash
cargo test -p aiy-adapter-gemini --offline
cargo test -p aiy-adapter-codex --offline
cargo test -p aiy-adapter-claude --offline
cargo test -p aiy-adapter-grok --offline
```

**Acceptance**:
- Adapters correctly use OAuth tokens when available
- Adapters fall back to API keys if OAuth not configured (backward compat)
- Refresh logic works (mock tests with expired tokens)
- Error messages never leak tokens

---

### Task 9: Add Cloud-Specific Adapters (Parallel)

**New Crates**:
- `crates/aiy-adapter-bedrock/` - AWS Bedrock (Anthropic via SigV4)
- `crates/aiy-adapter-azure-openai/` - Azure OpenAI (AAD tokens)
- `crates/aiy-adapter-vertex/` - GCP Vertex AI (Google OAuth, multi-model)

**Example**: `crates/aiy-adapter-bedrock/src/lib.rs`

```rust
// NOTE: Pseudo-code: use the official AWS Rust SDK for Bedrock Runtime.
// The SDK handles SigV4 signing automatically when constructed from `aws_config`.
use aws_sdk_bedrockruntime::Client as BedrockClient;

pub struct BedrockAdapter {
    client: BedrockClient,
    model_id: String,
}

impl BedrockAdapter {
    pub async fn new(region: &str, model_id: &str) -> anyhow::Result<Self> {
        let config = aws_config::load_from_env().await;
        let client = BedrockClient::new(&config);

        Ok(Self {
            client,
            model_id: model_id.to_string(),
        })
    }

    pub async fn invoke(&self, prompt: &str) -> anyhow::Result<String> {
        let request_body = json!({
            "prompt": prompt,
            "max_tokens_to_sample": 1024,
        });

        let response = self.client
            .invoke_model()
            .model_id(&self.model_id)
            .body(Blob::new(serde_json::to_vec(&request_body)?))
            .send()
            .await?;

        let response_body: serde_json::Value = serde_json::from_slice(response.body.as_ref())?;

        Ok(response_body["completion"]
            .as_str()
            .unwrap_or("")
            .to_string())
    }
}

#[async_trait]
impl AgentAdapter for BedrockAdapter {
    async fn call(&self, prompt: &str) -> anyhow::Result<AgentResponse> {
        let text = self.invoke(prompt).await?;
        Ok(AgentResponse {
            agent_id: "claude".to_string(),
            text,
            metadata: Default::default(),
        })
    }
}
```

**Validation**:
```bash
cargo test -p aiy-adapter-bedrock --offline
cargo test -p aiy-adapter-azure-openai --offline
cargo test -p aiy-adapter-vertex --offline
```

**Acceptance**:
- Bedrock adapter signs requests with AWS SigV4
- Azure adapter attaches AAD tokens correctly
- Vertex adapter uses Google OAuth tokens
- All adapters work with mock transports in tests

---

### Task 10: Extend Config for Auth Modes (Sequential)

**Files**:
- `crates/aiy-core/src/config/pipeline.rs` (UPDATE `PipelineConfig`)
- `crates/aiy-cli/src/commands/config.rs` (UPDATE to support setting new keys)

**Add fields**:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    // ... existing fields ...

    /// Auth broker URL for providers without OAuth (optional)
    pub auth_broker_url: Option<String>,

    /// Per-provider auth configuration
    pub provider_auth: HashMap<String, ProviderAuthConfig>,

    /// Default auth mode enforcement
    pub auth_mode_enforcement: AuthEnforcement,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderAuthConfig {
    /// Preferred auth mode (direct, azure, bedrock, vertex, broker)
    pub mode: String,

    /// Provider-specific settings (e.g., Azure tenant ID, AWS region)
    pub settings: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuthEnforcement {
    /// Require OAuth/IAM, fail if not available
    Strict,

    /// Prefer OAuth/IAM, fall back to API keys with warning
    Prefer,

    /// Allow API keys without warning (legacy mode)
    Allow,
}
```

**Example config** (`~/.config/all-in-yum/config.toml`):
```toml
auth_mode_enforcement = "Prefer"
auth_broker_url = "https://auth.company.com"

[provider_auth.openai]
mode = "azure"
settings = { tenant_id = "...", resource = "..." }

[provider_auth.anthropic]
mode = "bedrock"
settings = { region = "us-east-1", model_id = "anthropic.claude-v2" }

[provider_auth.google]
mode = "direct"
settings = { project_id = "my-gcp-project" }
```

**Update CLI config command** (`crates/aiy-cli/src/commands/config.rs`):
- Add support for setting:
  - `auth_broker_url`
  - `auth_mode_enforcement` (`Strict|Prefer|Allow`)
  - `provider_auth.<provider>.mode` (e.g., `provider_auth.openai.mode=azure`)
  - `provider_auth.<provider>.settings.<key>` (e.g., `provider_auth.openai.settings.tenant_id=<...>`)
- Treat `<provider>` as the credential provider ID used by the credential store: `xai`, `anthropic`, `google`, `openai` (not agent names like `grok/claude/codex`).
- Keep the “no secrets in config” invariant: config stores only non-sensitive settings (URLs, tenant IDs, project IDs), never tokens/keys.

**Decision Point**: **Enforce "no manual API keys" by default?**

**Recommendation**: **Use `AuthEnforcement::Prefer` as default**

**Rationale**:
- Transition period: existing users need time to migrate
- Graceful degradation: if OAuth fails, fall back to API key with warning
- Enterprise can set `Strict` via config management

**Behavior**:
- `Strict`: API key prompts fail with error directing to `aiy auth login`
- `Prefer`: API key prompts show warning: "⚠️  API keys are deprecated. Use `aiy auth login <provider>` for SSO-backed auth."
- `Allow`: No warnings (legacy compat)

**Validation**:
```bash
cargo test -p aiy-core config
```

**Acceptance**:
- Config loads auth settings correctly
- Auth enforcement modes behave as specified
- Missing config fields use sane defaults

---

### Task 11: Update Security & Redaction (Sequential)

**File**: `crates/aiy-core/src/security/sanitization.rs`

**Add OAuth token patterns**:
```rust
lazy_static! {
    static ref OAUTH_TOKEN_PATTERNS: Vec<Regex> = vec![
        // JWT tokens (three base64 segments separated by dots)
        Regex::new(r"ey[A-Za-z0-9_-]+\.ey[A-Za-z0-9_-]+\.[A-Za-z0-9_-]+").unwrap(),

        // Azure AD tokens (ya29.* for Google, similar patterns)
        Regex::new(r"ya29\.[A-Za-z0-9_-]{20,}").unwrap(),

        // Generic Bearer tokens
        Regex::new(r"Bearer\s+[A-Za-z0-9_\-\.+]{20,}").unwrap(),
    ];
}

pub fn sanitize_oauth_tokens(text: &str) -> String {
    let mut result = text.to_string();
    for pattern in OAUTH_TOKEN_PATTERNS.iter() {
        result = pattern.replace_all(&result, "[TOKEN_REDACTED]").to_string();
    }
    result
}
```

**Update main sanitization**:
```rust
pub fn sanitize_sensitive_data(text: &str) -> String {
    let mut result = text.to_string();

    // Existing API key redaction
    result = sanitize_api_keys(&result);

    // Add OAuth token redaction
    result = sanitize_oauth_tokens(&result);

    result
}
```

**Validation**:
```bash
cargo test -p aiy-core sanitization
```

**Add test**:
```rust
#[test]
fn test_oauth_token_redaction() {
    let jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
    let sanitized = sanitize_oauth_tokens(jwt);
    assert_eq!(sanitized, "[TOKEN_REDACTED]");

    let bearer = "Authorization: Bearer ya29.a0AfH6SMBx...";
    let sanitized = sanitize_oauth_tokens(bearer);
    assert!(sanitized.contains("[TOKEN_REDACTED]"));
    assert!(!sanitized.contains("ya29"));
}
```

**Acceptance**:
- JWT patterns correctly detected and redacted
- OAuth token never appears in sanitized output
- Existing API key redaction still works

---

### Task 12: Add Offline CLI Smoke Tests for OAuth (Sequential)

**File**: `crates/aiy-cli/tests/cli_auth_tests.rs` (NEW)

```rust
#![cfg(not(feature = "http"))]

use std::process::Command;
use tempfile::TempDir;

fn aiy_bin() -> std::path::PathBuf {
    std::env::var_os("CARGO_BIN_EXE_aiy")
        .map(std::path::PathBuf::from)
        .expect("CARGO_BIN_EXE_aiy not set")
}

#[test]
fn test_auth_status_shows_oauth_token() {
    let env = TestEnv::new();

    // Seed mock OAuth token
    env.seed_oauth_token("google", OAuthToken {
        access_token: "ya29.MOCK_TOKEN".to_string(),
        refresh_token: Some("1//MOCK_REFRESH".to_string()),
        token_type: "Bearer".to_string(),
        expires_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64 + 3600,
        scopes: vec!["https://www.googleapis.com/auth/cloud-platform".to_string()],
        issuer: Some("https://accounts.google.com".to_string()),
        metadata: HashMap::new(),
    });

    let mut cmd = env.cmd();
    cmd.args(["auth", "status"]);
    let output = cmd.output().expect("Failed to execute aiy");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should show OAuth status
    assert!(stdout.contains("GOOGLE"));
    assert!(stdout.contains("VALID") || stdout.contains("✅"));
    assert!(stdout.contains("OAuth"));

    // Should NOT show the actual token
    assert!(!stdout.contains("ya29.MOCK_TOKEN"));
    assert!(!stdout.contains("1//MOCK_REFRESH"));
}

#[test]
fn test_auth_logout_clears_tokens() {
    let env = TestEnv::new();
    env.seed_oauth_token("google", mock_token());

    // Logout
    let mut cmd = env.cmd();
    cmd.args(["auth", "logout", "google"]);
    let output = cmd.output().expect("Failed to execute aiy");
    assert!(output.status.success());

    // Verify token removed
    let mut status_cmd = env.cmd();
    status_cmd.args(["auth", "status", "google"]);
    let status_output = status_cmd.output().expect("Failed to execute aiy");

    let stdout = String::from_utf8_lossy(&status_output.stdout);
    assert!(stdout.contains("NOT CONFIGURED") || stdout.contains("❌"));
}

#[test]
fn test_oauth_tokens_never_in_error_output() {
    let env = TestEnv::new();
    env.seed_oauth_token("google", mock_token());

    // Trigger an error (e.g., invalid command)
    let mut cmd = env.cmd();
    cmd.args(["auth", "invalid-subcommand"]);
    let output = cmd.output().expect("Failed to execute aiy");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Tokens must be redacted in errors
    assert!(!stderr.contains("ya29.MOCK_TOKEN"));
    assert!(!stderr.contains("1//MOCK_REFRESH"));
}

#[test]
fn test_auth_login_device_code_flow_mock() {
    // Do NOT bind TCP ports in tests (some environments prohibit localhost binding).
    //
    // Instead:
    // - Inject a mock `OAuthHttpClient`/transport into the auth strategy, and
    // - Unit-test device-code polling as a pure state machine with canned responses.
}
```

**Validation**:
```bash
cargo test -p aiy-cli --test cli_auth_tests --offline
```

**Acceptance**:
- OAuth token status displayed correctly
- Tokens never appear in output (sanitization working)
- Logout clears credentials
- Tests run offline without network

---

## Validation Matrix

After all tasks complete, run:

```bash
# Format check
cargo fmt --all -- --check

# Clippy (no warnings)
cargo clippy --workspace --all-targets -- -D warnings

# All workspace tests offline
cargo test --workspace --offline

# CLI-specific tests
cargo test -p aiy-cli --offline

# CLI with HTTP feature (should not trigger network in tests)
cargo test -p aiy-cli --features http --offline

# Auth smoke tests
cargo test -p aiy-cli --test cli_auth_tests --offline

# Security tests (token redaction)
cargo test -p aiy-core sanitization
```

**All must pass with zero failures.**

---

## Acceptance Criteria

1. ✅ **OAuth Commands Work**:
   - `aiy auth login google` initiates device code flow
   - `aiy auth login openai --via azure` initiates Azure AD flow
   - `aiy auth login anthropic --via bedrock` guides AWS credential setup
   - `aiy auth status` shows token expiry and refresh status
   - `aiy auth logout <provider>` clears credentials

2. ✅ **Token Management**:
   - Tokens stored encrypted (keychain or encrypted file)
   - Automatic refresh when expired (< 5 min remaining)
   - Expiry detection works correctly
   - Refresh tokens used to obtain new access tokens

3. ✅ **Cloud IAM Integration**:
   - Google OAuth device flow works (with mock IdP in tests)
   - Azure AD device flow works (with mock IdP in tests)
   - AWS Bedrock detects credentials from AWS SDK
   - Vertex AI uses Google OAuth tokens

4. ✅ **Auth Broker Support**:
   - CLI can authenticate via configured broker URL
   - Broker device code flow works (with mock broker in tests)
   - Broker tokens stored and refreshed

5. ✅ **Adapter Updates**:
   - All adapters use `Credential` abstraction
   - Adapters refresh tokens before requests
   - Adapters fall back to API keys if OAuth not configured (backward compat)
   - Error messages guide users to re-authenticate

6. ✅ **Security Invariants**:
   - No tokens in stdout/stderr (sanitization tests pass)
   - No tokens in config files (only in encrypted storage)
   - Tokens zeroized from memory after use
   - All tests with dummy tokens pass redaction checks

7. ✅ **Offline Testing**:
   - All tests run with `--offline` flag
   - Mock OAuth servers for device code flows
   - No network required for test suite
   - `--features http` tests remain offline-safe

8. ✅ **Config Extension**:
   - Per-provider auth mode configuration works
   - Auth enforcement modes (Strict/Prefer/Allow) behave correctly
   - Auth broker URL configurable

9. ✅ **Backward Compatibility**:
   - Existing API key workflows still work (with deprecation warnings if `Prefer` mode)
   - `aiy credentials` commands still function
   - No breaking changes to config format (new fields optional)

10. ✅ **Documentation**:
    - Migration guide written (how to move from API keys to OAuth)
    - Enterprise setup guide written (how to deploy auth broker, configure Azure/AWS)
    - User runbook written (how to authenticate, troubleshoot)

---

## Risk Notes

### Token Refresh Edge Cases
- **Risk**: Token expires during long-running operation
- **Mitigation**: Check expiry at start of operation; for streaming responses, ensure token valid for at least operation duration estimate; accept that mid-operation expiry may cause failure (user must re-run)

### Cloud CLI Dependencies
- **Risk**: AWS/Azure SDKs add significant dependencies
- **Mitigation**: Use workspace dependencies to deduplicate; only link SDKs in cloud-specific adapter crates, not core

### Provider OAuth Unavailability
- **Risk**: xAI remains without OAuth or cloud alternative
- **Mitigation**: Broker integration provides path; document API key as interim (with `AuthEnforcement::Allow`)

### User Confusion
- **Risk**: Multiple auth methods per provider confuse users
- **Mitigation**: Provide `aiy auth setup` wizard that detects environment (Azure/AWS/GCP presence) and recommends best path; default to simplest (API key) if detection fails, with upgrade prompt

### Secret Storage Portability
- **Risk**: Keychain not available on all platforms (e.g., headless Linux)
- **Mitigation**: Encrypted file fallback already implemented; ensure AIY_CREDENTIALS_PASSWORD env var works for automation

### Offline Constraints
- **Risk**: OAuth flows inherently require network
- **Mitigation**: For testing, use mock HTTP servers or seed tokens directly; for real usage, accept that login requires network (subsequent calls can be offline if tokens cached)

---

## Suggested Commit Breakdown

### Commit 1: OAuth Token Infrastructure
```
feat(core): Add OAuth token infrastructure

- Define OAuthToken struct with expiry, refresh, metadata
- Create Credential enum for API key vs OAuth abstraction
- Implement AuthStrategy trait for device code/PKCE flows
- Add generic OAuth HTTP client for token exchange

Files: crates/aiy-core/src/security/token.rs (NEW)
       crates/aiy-core/src/security/credential.rs (NEW)
       crates/aiy-core/src/auth/strategy.rs (NEW)
```

### Commit 2: Extend CredentialManager for OAuth
```
feat(core): Extend CredentialManager to store and refresh OAuth tokens

- Add store_token() and get_token() methods
- Implement refresh_token() with OAuth refresh flow
- Ensure tokens zeroized from memory
- Preserve API key storage for backward compat

Files: crates/aiy-core/src/security/credential_manager.rs
```

### Commit 3: Implement Google OAuth Strategy
```
feat(auth): Implement Google OAuth device code flow

- Add GoogleOAuthStrategy for Gemini/Vertex AI
- Support device authorization grant (RFC 8628)
- Implement token polling and refresh
- Add tests with mock OAuth transport (no TCP binding)

Files: crates/aiy-core/src/auth/strategies/google.rs (NEW)
```

### Commit 4: Implement Azure AD Strategy
```
feat(auth): Implement Azure AD device code flow for OpenAI

- Add AzureAdStrategy for Azure OpenAI
- Support multi-tenant configuration
- Scope tokens correctly for Cognitive Services
- Add tests with mock OAuth transport (no TCP binding)

Files: crates/aiy-core/src/auth/strategies/azure.rs (NEW)
```

### Commit 5: Implement AWS Bedrock Strategy
```
feat(auth): Implement AWS Bedrock IAM integration

- Add BedrockStrategy using AWS SDK
- Detect credentials from env/SSO/file
- Support SigV4 signing for Bedrock calls
- Add tests with mock AWS credentials

Files: crates/aiy-core/src/auth/strategies/bedrock.rs (NEW)
```

### Commit 6: Implement Auth Broker Strategy
```
feat(auth): Add auth broker integration for xAI

- Add AuthBrokerStrategy for enterprise broker
- Support device code flow to broker endpoints
- Document broker API contract (OpenAPI spec)
- Add tests with mock broker transport (no TCP binding)

Files: crates/aiy-core/src/auth/strategies/broker.rs (NEW)
       docs/auth-broker-api.yaml (NEW)
```

### Commit 7: Add CLI Auth Commands
```
feat(cli): Add auth login/status/logout commands

- Implement `aiy auth login <provider> [--via <mode>]`
- Implement `aiy auth status` with token expiry display
- Implement `aiy auth logout <provider>`
- Support device code and browser-based flows

Files: crates/aiy-cli/src/commands/auth.rs (NEW)
       crates/aiy-cli/src/main.rs (register auth subcommand)
```

### Commit 8: Update Adapters to Use Credentials
```
refactor(adapters): Use Credential abstraction instead of API keys

- Replace get_key() with get_credential() in all adapters
- Add automatic token refresh before requests
- Handle expiry gracefully with retry logic
- Preserve API key fallback for backward compat

Files: crates/aiy-adapter-*/src/client.rs
```

### Commit 9: Add Cloud-Specific Adapters
```
feat(adapters): Add Bedrock, Azure OpenAI, and Vertex AI adapters

- Implement aiy-adapter-bedrock for AWS Bedrock (SigV4)
- Implement aiy-adapter-azure-openai for Azure OpenAI (AAD)
- Implement aiy-adapter-vertex for GCP Vertex AI (OAuth)
- Add tests with mock cloud endpoints

Files: crates/aiy-adapter-bedrock/ (NEW)
       crates/aiy-adapter-azure-openai/ (NEW)
       crates/aiy-adapter-vertex/ (NEW)
```

### Commit 10: Extend Config for Auth Modes
```
feat(config): Add per-provider auth configuration

- Add auth_broker_url field
- Add provider_auth map for per-provider settings
- Add AuthEnforcement enum (Strict/Prefer/Allow)
- Default to Prefer mode for migration path

Files: crates/aiy-core/src/config/pipeline.rs
```

### Commit 11: Enhance Security & Redaction
```
security(core): Add OAuth token redaction patterns

- Add JWT regex patterns to sanitization
- Add Azure/Google token patterns
- Ensure tokens never in logs/errors
- Add comprehensive redaction tests

Files: crates/aiy-core/src/security/sanitization.rs
```

### Commit 12: Add OAuth CLI Smoke Tests
```
test(cli): Add comprehensive OAuth auth smoke tests

- Test auth status with mock tokens
- Test auth logout clears credentials
- Test token redaction in error output
- Ensure all tests run offline

Files: crates/aiy-cli/tests/cli_auth_tests.rs (NEW)
```

### Commit 13: Documentation
```
docs: Add OAuth migration and enterprise setup guides

- Migration guide: API keys → OAuth
- Enterprise setup: broker deployment
- Cloud integration: AWS/Azure/GCP setup
- Troubleshooting guide

Files: docs/auth-migration-guide.md (NEW)
       docs/enterprise-auth-setup.md (NEW)
       docs/cloud-iam-integration.md (NEW)
```

---

## Human Live Test Runbook

After automated tests pass, perform manual live testing:

### Prerequisites
```bash
# Build with HTTP feature enabled
cargo build -p aiy-cli --release --features http

# Verify binary
./target/release/aiy version
```

### Test 1: Google OAuth (Direct)

```bash
# Configure for Google
./target/release/aiy config set provider_auth.google.mode direct
./target/release/aiy config set provider_auth.google.settings.project_id YOUR_GCP_PROJECT

# Login
./target/release/aiy auth login google
# Follow device code instructions, complete in browser

# Verify status
./target/release/aiy auth status google
# Should show: ✅ VALID, OAuth, expires_at

# Test API call (if Gemini configured)
./target/release/aiy ask --agent gemini --prompt "Hello, test OAuth"

# Logout
./target/release/aiy auth logout google
```

### Test 2: Azure OpenAI

```bash
# Configure for Azure
./target/release/aiy config set provider_auth.openai.mode azure
./target/release/aiy config set provider_auth.openai.settings.tenant_id YOUR_TENANT_ID
./target/release/aiy config set provider_auth.openai.settings.resource YOUR_AZURE_RESOURCE

# Login
./target/release/aiy auth login openai --via azure
# Complete device code flow with Azure AD

# Verify
./target/release/aiy auth status openai
# Should show Azure AD token

# Test
./target/release/aiy ask --agent codex --prompt "Test Azure auth"
```

### Test 3: AWS Bedrock (Anthropic)

```bash
# Ensure AWS credentials configured
aws sso login --profile YOUR_PROFILE
# OR: aws configure

# Configure for Bedrock
./target/release/aiy config set provider_auth.anthropic.mode bedrock
./target/release/aiy config set provider_auth.anthropic.settings.region us-east-1

# Login (should detect existing AWS creds)
./target/release/aiy auth login anthropic --via bedrock
# Should confirm: "AWS credentials detected"

# Test
./target/release/aiy ask --agent claude --prompt "Test Bedrock"
```

### Test 4: Auth Broker (xAI)

```bash
# Deploy test broker (separate setup, not in scope)
# Configure broker URL
./target/release/aiy config set auth_broker_url https://auth.company.com

# Login
./target/release/aiy auth login xai
# Complete device code via broker

# Test
./target/release/aiy ask --agent grok --prompt "Test broker auth"
```

### Test 5: Token Refresh

```bash
# Wait for token near expiry (or mock expiry)
# Trigger API call - should auto-refresh
./target/release/aiy ask --agent google --prompt "Test refresh"

# Check status - should show new expiry time
./target/release/aiy auth status google
```

### Test 6: Security - Token Leakage

```bash
# Trigger error with token present
./target/release/aiy auth status --invalid-flag 2>&1 | grep -i "ya29\|ey[A-Z]"
# Should return nothing (tokens redacted)

# Check logs (if any)
# (If the CLI writes logs to disk in your environment, ensure they are sanitized.)
cat ~/.config/all-in-yum/aiy.log 2>/dev/null | grep -i "bearer\\|oauth"
# Should show [TOKEN_REDACTED] or no sensitive data
```

### Expected Results
- All auth flows complete without errors
- Tokens stored securely (verify not in `~/.config/all-in-yum/config.toml`)
- API calls succeed with OAuth tokens
- Status command shows expiry and refresh capability
- No tokens visible in any output or logs

---

## Critical Files for Implementation

**Core**:
- `crates/aiy-core/src/security/token.rs` - OAuthToken struct (NEW)
- `crates/aiy-core/src/security/credential.rs` - Credential enum (NEW)
- `crates/aiy-core/src/security/credential_manager.rs` - OAuth methods (UPDATE)
- `crates/aiy-core/src/auth/strategy.rs` - AuthStrategy trait (NEW)
- `crates/aiy-core/src/auth/strategies/google.rs` - Google OAuth (NEW)
- `crates/aiy-core/src/auth/strategies/azure.rs` - Azure AD (NEW)
- `crates/aiy-core/src/auth/strategies/bedrock.rs` - AWS Bedrock (NEW)
- `crates/aiy-core/src/auth/strategies/broker.rs` - Auth broker (NEW)
- `crates/aiy-core/src/config/pipeline.rs` - Auth config fields (UPDATE)

**CLI**:
- `crates/aiy-cli/src/commands/auth.rs` - Auth commands (NEW)
- `crates/aiy-cli/src/main.rs` - Register auth subcommand (UPDATE)

**Adapters**:
- `crates/aiy-adapter-gemini/src/client.rs` - Use Credential (UPDATE)
- `crates/aiy-adapter-grok/src/client.rs` - Use Credential (UPDATE)
- `crates/aiy-adapter-claude/src/client.rs` - Use Credential (UPDATE)
- `crates/aiy-adapter-codex/src/client.rs` - Use Credential (UPDATE)
- `crates/aiy-adapter-bedrock/` - New crate for AWS Bedrock (NEW)
- `crates/aiy-adapter-azure-openai/` - New crate for Azure (NEW)
- `crates/aiy-adapter-vertex/` - New crate for Vertex AI (NEW)

**Tests**:
- `crates/aiy-cli/tests/cli_auth_tests.rs` - OAuth smoke tests (NEW)
- `crates/aiy-core/tests/oauth_token_tests.rs` - Token management tests (NEW)

**Docs**:
- `docs/auth-migration-guide.md` - User migration guide (NEW)
- `docs/enterprise-auth-setup.md` - Enterprise deployment (NEW)
- `docs/auth-broker-api.yaml` - Broker API spec (NEW)
