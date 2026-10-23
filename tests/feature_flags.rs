//! Feature flag implementation and testing
//!
//! This module tests conditional compilation features and runtime
//! feature detection capabilities of the LSP Bridge.

use lsp_bridge::LspClientCapabilities;
use lsp_types::*;
use std::collections::HashMap;

/// Feature capability detector and manager
#[derive(Debug, Clone)]
pub struct FeatureManager {
    enabled_features: HashMap<String, bool>,
    server_capabilities: Option<ServerCapabilities>,
    client_capabilities: LspClientCapabilities,
}

impl FeatureManager {
    pub fn new() -> Self {
        Self {
            enabled_features: HashMap::new(),
            server_capabilities: None,
            client_capabilities: LspClientCapabilities::default(),
        }
    }

    /// Enable or disable a feature at runtime
    pub fn set_feature(&mut self, feature: &str, enabled: bool) {
        self.enabled_features.insert(feature.to_string(), enabled);
    }

    /// Check if a feature is enabled
    pub fn is_feature_enabled(&self, feature: &str) -> bool {
        self.enabled_features.get(feature).copied().unwrap_or(false)
    }

    /// Update server capabilities and detect supported features
    pub fn update_server_capabilities(&mut self, capabilities: ServerCapabilities) {
        self.server_capabilities = Some(capabilities.clone());

        // Auto-detect features based on server capabilities
        if let Some(_completion_provider) = &capabilities.completion_provider {
            self.set_feature("completion", true);
        }

        if capabilities.hover_provider.is_some() {
            self.set_feature("hover", true);
        }

        if capabilities.definition_provider.is_some() {
            self.set_feature("goto_definition", true);
        }

        if capabilities.references_provider.is_some() {
            self.set_feature("find_references", true);
        }

        if capabilities.document_formatting_provider.is_some() {
            self.set_feature("formatting", true);
        }

        if capabilities.rename_provider.is_some() {
            self.set_feature("rename", true);
        }

        if capabilities.code_action_provider.is_some() {
            self.set_feature("code_actions", true);
        }

        if capabilities.workspace_symbol_provider.is_some() {
            self.set_feature("workspace_symbols", true);
        }

        if capabilities.document_symbol_provider.is_some() {
            self.set_feature("document_symbols", true);
        }
    }

    /// Get list of enabled features
    pub fn get_enabled_features(&self) -> Vec<String> {
        self.enabled_features
            .iter()
            .filter_map(
                |(feature, &enabled)| {
                    if enabled { Some(feature.clone()) } else { None }
                },
            )
            .collect()
    }

    /// Check compatibility between client and server capabilities
    pub fn check_compatibility(&self) -> CompatibilityReport {
        let mut compatible_features = Vec::new();
        let mut missing_server_features = Vec::new();
        let mut missing_client_features = Vec::new();

        // Define the features we want to check
        let desired_features = vec![
            "completion",
            "hover",
            "goto_definition",
            "find_references",
            "formatting",
            "rename",
            "code_actions",
            "workspace_symbols",
            "document_symbols",
        ];

        for feature in desired_features {
            let server_supports = self.is_feature_enabled(feature);
            let client_supports = self.client_supports_feature(feature);

            if server_supports && client_supports {
                compatible_features.push(feature.to_string());
            } else if !server_supports && client_supports {
                missing_server_features.push(feature.to_string());
            } else if server_supports && !client_supports {
                missing_client_features.push(feature.to_string());
            }
        }

        CompatibilityReport {
            compatible_features,
            missing_server_features,
            missing_client_features,
        }
    }

    /// Check if client supports a specific feature
    fn client_supports_feature(&self, feature: &str) -> bool {
        match feature {
            "completion" => self.client_capabilities.text_document.completion.is_some(),
            "hover" => self.client_capabilities.text_document.hover.is_some(),
            "goto_definition" => self.client_capabilities.text_document.definition.is_some(),
            "find_references" => self.client_capabilities.text_document.references.is_some(),
            "formatting" => self.client_capabilities.text_document.formatting.is_some(),
            "rename" => self.client_capabilities.text_document.rename.is_some(),
            "code_actions" => self.client_capabilities.text_document.code_action.is_some(),
            "workspace_symbols" => self.client_capabilities.workspace.symbol.is_some(),
            "document_symbols" => self
                .client_capabilities
                .text_document
                .document_symbol
                .is_some(),
            _ => false,
        }
    }
}

impl Default for FeatureManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Compatibility report between client and server capabilities
#[derive(Debug, Clone)]
pub struct CompatibilityReport {
    pub compatible_features: Vec<String>,
    pub missing_server_features: Vec<String>,
    pub missing_client_features: Vec<String>,
}

impl CompatibilityReport {
    pub fn is_fully_compatible(&self) -> bool {
        self.missing_server_features.is_empty() && self.missing_client_features.is_empty()
    }

    pub fn compatibility_score(&self) -> f64 {
        let total_features = self.compatible_features.len()
            + self.missing_server_features.len()
            + self.missing_client_features.len();

        if total_features == 0 {
            return 1.0;
        }

        self.compatible_features.len() as f64 / total_features as f64
    }
}

/// Test conditional compilation features
#[test]
fn test_conditional_compilation_features() {
    println!("🏗️ Testing conditional compilation features...");

    // Test that default features are enabled
    #[cfg(feature = "default")]
    {
        println!("✅ Default features are enabled");
    }

    #[cfg(not(feature = "default"))]
    {
        println!("⚠️ Default features are disabled");
    }

    // Test diagnostics feature
    #[cfg(feature = "diagnostics")]
    {
        println!("✅ Diagnostics feature is enabled");
        let _diagnostics_enabled = true;
    }

    #[cfg(not(feature = "diagnostics"))]
    {
        println!("⚠️ Diagnostics feature is disabled");
        let _diagnostics_enabled = false;
    }

    // Test completion feature
    #[cfg(feature = "completion")]
    {
        println!("✅ Completion feature is enabled");
        let _completion_enabled = true;
    }

    #[cfg(not(feature = "completion"))]
    {
        println!("⚠️ Completion feature is disabled");
        let _completion_enabled = false;
    }

    // Test formatting feature
    #[cfg(feature = "formatting")]
    {
        println!("✅ Formatting feature is enabled");
        let _formatting_enabled = true;
    }

    #[cfg(not(feature = "formatting"))]
    {
        println!("⚠️ Formatting feature is disabled");
        let _formatting_enabled = false;
    }

    // Test optional features
    #[cfg(feature = "references")]
    {
        println!("✅ References feature is enabled");
    }

    #[cfg(feature = "definitions")]
    {
        println!("✅ Definitions feature is enabled");
    }

    #[cfg(feature = "hover")]
    {
        println!("✅ Hover feature is enabled");
    }

    #[cfg(feature = "rename")]
    {
        println!("✅ Rename feature is enabled");
    }

    #[cfg(feature = "code-actions")]
    {
        println!("✅ Code actions feature is enabled");
    }

    #[cfg(feature = "workspace-symbols")]
    {
        println!("✅ Workspace symbols feature is enabled");
    }

    #[cfg(feature = "document-symbols")]
    {
        println!("✅ Document symbols feature is enabled");
    }

    #[cfg(feature = "semantic-tokens")]
    {
        println!("✅ Semantic tokens feature is enabled");
    }

    println!("✅ Conditional compilation test completed");
}

/// Test runtime feature detection
#[tokio::test]
async fn test_runtime_feature_detection() {
    let mut feature_manager = FeatureManager::new();

    println!("🔍 Testing runtime feature detection...");

    // Initially no features should be enabled
    let initial_features = feature_manager.get_enabled_features();
    assert!(
        initial_features.is_empty(),
        "No features should be enabled initially"
    );

    // Manually enable some features
    feature_manager.set_feature("completion", true);
    feature_manager.set_feature("hover", true);
    feature_manager.set_feature("formatting", false);

    assert!(feature_manager.is_feature_enabled("completion"));
    assert!(feature_manager.is_feature_enabled("hover"));
    assert!(!feature_manager.is_feature_enabled("formatting"));
    assert!(!feature_manager.is_feature_enabled("nonexistent"));

    let enabled_features = feature_manager.get_enabled_features();
    assert_eq!(enabled_features.len(), 2);
    assert!(enabled_features.contains(&"completion".to_string()));
    assert!(enabled_features.contains(&"hover".to_string()));

    println!("✅ Runtime feature detection test passed");
    println!("   Enabled features: {enabled_features:?}");
}

/// Test server capability detection
#[tokio::test]
async fn test_server_capability_detection() {
    let mut feature_manager = FeatureManager::new();

    println!("🖥️ Testing server capability detection...");

    // Create mock server capabilities
    let server_capabilities = ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        completion_provider: Some(CompletionOptions::default()),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        signature_help_provider: None,
        definition_provider: Some(OneOf::Left(true)),
        type_definition_provider: None,
        implementation_provider: None,
        references_provider: Some(OneOf::Left(true)),
        document_highlight_provider: None,
        document_symbol_provider: Some(OneOf::Left(true)),
        workspace_symbol_provider: Some(OneOf::Left(true)),
        code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
        code_lens_provider: None,
        document_formatting_provider: Some(OneOf::Left(true)),
        document_range_formatting_provider: None,
        document_on_type_formatting_provider: None,
        rename_provider: Some(OneOf::Left(true)),
        document_link_provider: None,
        color_provider: None,
        folding_range_provider: None,
        declaration_provider: None,
        execute_command_provider: None,
        workspace: None,
        call_hierarchy_provider: None,
        semantic_tokens_provider: None,
        moniker_provider: None,
        linked_editing_range_provider: None,
        inline_value_provider: None,
        inlay_hint_provider: None,
        diagnostic_provider: None,
        notebook_document_sync: None,
        experimental: None,
        position_encoding: None,
        selection_range_provider: None,
    };

    // Update feature manager with capabilities
    feature_manager.update_server_capabilities(server_capabilities);

    // Verify features were detected
    assert!(feature_manager.is_feature_enabled("completion"));
    assert!(feature_manager.is_feature_enabled("hover"));
    assert!(feature_manager.is_feature_enabled("goto_definition"));
    assert!(feature_manager.is_feature_enabled("find_references"));
    assert!(feature_manager.is_feature_enabled("formatting"));
    assert!(feature_manager.is_feature_enabled("rename"));
    assert!(feature_manager.is_feature_enabled("code_actions"));
    assert!(feature_manager.is_feature_enabled("workspace_symbols"));
    assert!(feature_manager.is_feature_enabled("document_symbols"));

    let enabled_features = feature_manager.get_enabled_features();

    println!("✅ Server capability detection test passed");
    println!("   Detected features: {enabled_features:?}");
    assert!(
        enabled_features.len() >= 8,
        "Should detect multiple features"
    );
}

/// Test client-server compatibility checking
#[tokio::test]
async fn test_compatibility_checking() {
    let mut feature_manager = FeatureManager::new();

    println!("🤝 Testing client-server compatibility...");

    // Configure client capabilities
    let mut client_caps = LspClientCapabilities::default();
    client_caps.text_document.completion = Some(Default::default());
    client_caps.text_document.hover = Some(Default::default());
    client_caps.text_document.definition = Some(Default::default());
    // Note: Not setting references capability to test missing client features

    feature_manager.client_capabilities = client_caps;

    // Configure server capabilities (enable more features than client supports)
    feature_manager.set_feature("completion", true);
    feature_manager.set_feature("hover", true);
    feature_manager.set_feature("goto_definition", true);
    feature_manager.set_feature("find_references", true); // Server supports but client doesn't
    feature_manager.set_feature("formatting", false); // Neither supports

    // Check compatibility
    let report = feature_manager.check_compatibility();

    println!("📊 Compatibility report:");
    println!("   Compatible features: {:?}", report.compatible_features);
    println!(
        "   Missing server features: {:?}",
        report.missing_server_features
    );
    println!(
        "   Missing client features: {:?}",
        report.missing_client_features
    );
    println!(
        "   Compatibility score: {:.2}",
        report.compatibility_score()
    );

    // Verify compatibility analysis
    assert!(
        report
            .compatible_features
            .contains(&"completion".to_string())
    );
    assert!(report.compatible_features.contains(&"hover".to_string()));
    assert!(
        report
            .compatible_features
            .contains(&"goto_definition".to_string())
    );

    // Should detect that client doesn't support references
    assert!(
        report
            .missing_client_features
            .contains(&"find_references".to_string())
    );

    // Compatibility score should be reasonable
    let score = report.compatibility_score();
    assert!(
        score > 0.5 && score < 1.0,
        "Compatibility score should be between 0.5 and 1.0, got {score}"
    );

    assert!(
        !report.is_fully_compatible(),
        "Should not be fully compatible due to missing client features"
    );

    println!("✅ Compatibility checking test passed");
}

/// Test feature negotiation during server initialization
#[tokio::test]
async fn test_feature_negotiation() {
    let mut feature_manager = FeatureManager::new();

    println!("🤹 Testing feature negotiation...");

    // Simulate the LSP initialization handshake

    // Step 1: Client declares its capabilities
    let mut client_caps = LspClientCapabilities::default();
    client_caps.text_document.completion = Some(Default::default());
    client_caps.text_document.hover = Some(Default::default());
    client_caps.text_document.definition = Some(Default::default());
    client_caps.text_document.references = Some(Default::default());
    client_caps.text_document.formatting = Some(Default::default());

    feature_manager.client_capabilities = client_caps;

    // Step 2: Server responds with its capabilities
    let server_capabilities = ServerCapabilities {
        completion_provider: Some(CompletionOptions::default()),
        hover_provider: Some(HoverProviderCapability::Simple(true)),
        definition_provider: Some(OneOf::Left(true)),
        // Note: Server doesn't support references or formatting
        references_provider: None,
        document_formatting_provider: None,
        ..Default::default()
    };

    feature_manager.update_server_capabilities(server_capabilities);

    // Step 3: Determine negotiated feature set
    let report = feature_manager.check_compatibility();
    let negotiated_features = report.compatible_features;

    println!("🤝 Negotiated features: {negotiated_features:?}");

    // Verify negotiation results
    assert!(negotiated_features.contains(&"completion".to_string()));
    assert!(negotiated_features.contains(&"hover".to_string()));
    assert!(negotiated_features.contains(&"goto_definition".to_string()));

    // These should not be in negotiated features
    assert!(!negotiated_features.contains(&"find_references".to_string()));
    assert!(!negotiated_features.contains(&"formatting".to_string()));

    // Test that only negotiated features are available
    for feature in &negotiated_features {
        assert!(feature_manager.is_feature_enabled(feature));
    }

    println!("✅ Feature negotiation test passed");
    println!(
        "   Successfully negotiated {} features",
        negotiated_features.len()
    );
}

/// Test feature flags in different build configurations
#[test]
fn test_build_configuration_features() {
    println!("⚙️ Testing build configuration features...");

    let mut feature_count = 0;
    let mut available_features = Vec::new();

    // Count available features based on compile-time flags
    #[cfg(feature = "diagnostics")]
    {
        feature_count += 1;
        available_features.push("diagnostics");
    }

    #[cfg(feature = "completion")]
    {
        feature_count += 1;
        available_features.push("completion");
    }

    #[cfg(feature = "formatting")]
    {
        feature_count += 1;
        available_features.push("formatting");
    }

    #[cfg(feature = "references")]
    {
        feature_count += 1;
        available_features.push("references");
    }

    #[cfg(feature = "definitions")]
    {
        feature_count += 1;
        available_features.push("definitions");
    }

    #[cfg(feature = "hover")]
    {
        feature_count += 1;
        available_features.push("hover");
    }

    #[cfg(feature = "rename")]
    {
        feature_count += 1;
        available_features.push("rename");
    }

    #[cfg(feature = "code-actions")]
    {
        feature_count += 1;
        available_features.push("code-actions");
    }

    #[cfg(feature = "workspace-symbols")]
    {
        feature_count += 1;
        available_features.push("workspace-symbols");
    }

    #[cfg(feature = "document-symbols")]
    {
        feature_count += 1;
        available_features.push("document-symbols");
    }

    #[cfg(feature = "semantic-tokens")]
    {
        feature_count += 1;
        available_features.push("semantic-tokens");
    }

    println!("📦 Build configuration:");
    println!("   Available features: {feature_count} ({available_features:?})");

    // With default features, we should have at least the core ones
    #[cfg(feature = "default")]
    {
        assert!(
            feature_count >= 3,
            "Default build should have at least 3 features, got {feature_count}"
        );
        assert!(
            available_features.contains(&"diagnostics"),
            "Default build should include diagnostics"
        );
        assert!(
            available_features.contains(&"completion"),
            "Default build should include completion"
        );
        assert!(
            available_features.contains(&"formatting"),
            "Default build should include formatting"
        );
    }

    println!("✅ Build configuration test passed");
}

/// Test runtime feature toggling
#[tokio::test]
async fn test_runtime_feature_toggling() {
    let mut feature_manager = FeatureManager::new();

    println!("🔄 Testing runtime feature toggling...");

    // Test dynamic feature enabling/disabling
    let features_to_test = vec!["completion", "hover", "formatting", "references"];

    for feature in &features_to_test {
        // Initially disabled
        assert!(!feature_manager.is_feature_enabled(feature));

        // Enable feature
        feature_manager.set_feature(feature, true);
        assert!(feature_manager.is_feature_enabled(feature));

        // Disable feature
        feature_manager.set_feature(feature, false);
        assert!(!feature_manager.is_feature_enabled(feature));

        // Re-enable
        feature_manager.set_feature(feature, true);
        assert!(feature_manager.is_feature_enabled(feature));
    }

    // Test that all features are enabled
    let enabled = feature_manager.get_enabled_features();
    assert_eq!(enabled.len(), features_to_test.len());

    for feature in &features_to_test {
        assert!(enabled.contains(&feature.to_string()));
    }

    // Test bulk disable
    for feature in &features_to_test {
        feature_manager.set_feature(feature, false);
    }

    let disabled = feature_manager.get_enabled_features();
    assert!(disabled.is_empty());

    println!("✅ Runtime feature toggling test passed");
    println!("   Tested {} features successfully", features_to_test.len());
}
