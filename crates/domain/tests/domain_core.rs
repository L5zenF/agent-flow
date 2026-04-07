use domain::{
    DomainError, GatewayCatalog, Model, ModelCatalog, ModelId, Provider, ProviderCatalog,
    ProviderId, RouteId, Workflow, WorkflowId, WorkflowIndex,
};

#[test]
fn typed_ids_require_non_blank_values() {
    assert_eq!(
        ProviderId::new(" ").unwrap_err(),
        DomainError::BlankProviderId
    );
    assert_eq!(ModelId::new("").unwrap_err(), DomainError::BlankModelId);
    assert_eq!(RouteId::new("\n\t").unwrap_err(), DomainError::BlankRouteId);
    assert!(WorkflowId::new("workflow-a").is_ok());
}

#[test]
fn typed_ids_trim_edges_but_preserve_casing_and_inner_characters() {
    let provider_id = ProviderId::new("  OpenAI-Primary  ").unwrap();

    assert_eq!(provider_id.as_str(), "OpenAI-Primary");
}

#[test]
fn workflow_index_requires_unique_ids() {
    let duplicate_id = WorkflowId::new("primary").unwrap();
    let workflows = vec![
        Workflow::new(duplicate_id.clone(), vec![RouteId::new("chat").unwrap()]).unwrap(),
        Workflow::new(duplicate_id, vec![RouteId::new("fallback").unwrap()]).unwrap(),
    ];

    let result = WorkflowIndex::new(workflows, None);

    assert_eq!(
        result.unwrap_err(),
        DomainError::DuplicateWorkflowId {
            workflow_id: WorkflowId::new("primary").unwrap(),
        }
    );
}

#[test]
fn workflow_index_requires_explicit_active_selection() {
    let primary = workflow("primary", &["chat"]);
    let backup = workflow("backup", &["fallback"]);

    let index = WorkflowIndex::new(vec![primary, backup], None).unwrap();

    assert_eq!(index.active(), None);
    assert_eq!(index.active_id(), None);
}

#[test]
fn workflow_index_rejects_missing_active_workflow() {
    let result = WorkflowIndex::new(
        vec![workflow("primary", &["chat"])],
        Some(WorkflowId::new("missing").unwrap()),
    );

    assert_eq!(
        result.unwrap_err(),
        DomainError::ActiveWorkflowNotFound {
            workflow_id: WorkflowId::new("missing").unwrap(),
        }
    );
}

#[test]
fn workflow_index_can_activate_existing_workflow() {
    let mut index = WorkflowIndex::new(
        vec![
            workflow("primary", &["chat"]),
            workflow("backup", &["fallback"]),
        ],
        None,
    )
    .unwrap();

    index.activate(&WorkflowId::new("backup").unwrap()).unwrap();

    assert_eq!(
        index.active().map(|workflow| workflow.id().as_str()),
        Some("backup")
    );
}

#[test]
fn workflow_index_rejects_activation_of_unknown_workflow() {
    let mut index = WorkflowIndex::new(vec![workflow("primary", &["chat"])], None).unwrap();

    let result = index.activate(&WorkflowId::new("missing").unwrap());

    assert_eq!(
        result.unwrap_err(),
        DomainError::ActiveWorkflowNotFound {
            workflow_id: WorkflowId::new("missing").unwrap(),
        }
    );
}

#[test]
fn workflow_index_rejects_active_selection_when_empty() {
    let result = WorkflowIndex::new(Vec::new(), Some(WorkflowId::new("primary").unwrap()));

    assert_eq!(
        result.unwrap_err(),
        DomainError::ActiveWorkflowDefinedWithoutWorkflows {
            workflow_id: WorkflowId::new("primary").unwrap(),
        }
    );
}

#[test]
fn workflow_index_active_is_none_for_empty_index() {
    let index = WorkflowIndex::new(Vec::new(), None).unwrap();

    assert_eq!(index.active(), None);
    assert_eq!(index.active_id(), None);
}

#[test]
fn provider_catalog_rejects_duplicate_provider_ids() {
    let providers = vec![
        Provider::new(ProviderId::new("openai").unwrap(), "OpenAI").unwrap(),
        Provider::new(ProviderId::new("openai").unwrap(), "Duplicate OpenAI").unwrap(),
    ];

    let result = ProviderCatalog::new(providers);

    assert_eq!(
        result.unwrap_err(),
        DomainError::DuplicateProviderId {
            provider_id: ProviderId::new("openai").unwrap(),
        }
    );
}

#[test]
fn model_catalog_allows_models_without_cross_catalog_validation() {
    let result = ModelCatalog::new(vec![
        Model::new(
            ModelId::new("gpt-4o").unwrap(),
            ProviderId::new("missing").unwrap(),
            "GPT-4o",
        )
        .unwrap(),
    ]);

    assert!(result.is_ok());
}

#[test]
fn model_catalog_rejects_duplicate_model_ids() {
    let result = ModelCatalog::new(vec![
        Model::new(
            ModelId::new("gpt-4o").unwrap(),
            ProviderId::new("openai").unwrap(),
            "GPT-4o",
        )
        .unwrap(),
        Model::new(
            ModelId::new("gpt-4o").unwrap(),
            ProviderId::new("openai").unwrap(),
            "GPT-4o Duplicate",
        )
        .unwrap(),
    ]);

    assert_eq!(
        result.unwrap_err(),
        DomainError::DuplicateModelId {
            model_id: ModelId::new("gpt-4o").unwrap(),
        }
    );
}

#[test]
fn workflow_requires_at_least_one_route() {
    let result = Workflow::new(WorkflowId::new("primary").unwrap(), Vec::new());

    assert_eq!(
        result.unwrap_err(),
        DomainError::EmptyWorkflowRoutes {
            workflow_id: WorkflowId::new("primary").unwrap(),
        }
    );
}

#[test]
fn provider_name_cannot_be_blank() {
    let result = Provider::new(ProviderId::new("openai").unwrap(), "   ");

    assert_eq!(result.unwrap_err(), DomainError::BlankProviderName);
}

#[test]
fn model_name_cannot_be_blank() {
    let result = Model::new(
        ModelId::new("gpt-4o").unwrap(),
        ProviderId::new("openai").unwrap(),
        "\t",
    );

    assert_eq!(result.unwrap_err(), DomainError::BlankModelName);
}

#[test]
fn gateway_catalog_enforces_provider_model_consistency() {
    let providers = ProviderCatalog::new(vec![
        Provider::new(ProviderId::new("openai").unwrap(), "OpenAI").unwrap(),
    ])
    .unwrap();
    let models = ModelCatalog::new(vec![
        Model::new(
            ModelId::new("gpt-4o").unwrap(),
            ProviderId::new("missing").unwrap(),
            "GPT-4o",
        )
        .unwrap(),
    ])
    .unwrap();

    let result = GatewayCatalog::new(providers, models);

    assert_eq!(
        result.unwrap_err(),
        DomainError::UnknownProviderReference {
            model_id: ModelId::new("gpt-4o").unwrap(),
            provider_id: ProviderId::new("missing").unwrap(),
        }
    );
}

#[test]
fn gateway_catalog_accepts_consistent_provider_model_sets() {
    let providers = ProviderCatalog::new(vec![
        Provider::new(ProviderId::new("openai").unwrap(), "OpenAI").unwrap(),
    ])
    .unwrap();
    let models = ModelCatalog::new(vec![
        Model::new(
            ModelId::new("gpt-4o").unwrap(),
            ProviderId::new("openai").unwrap(),
            "GPT-4o",
        )
        .unwrap(),
    ])
    .unwrap();

    let catalog = GatewayCatalog::new(providers, models).unwrap();

    assert!(
        catalog
            .providers()
            .contains(&ProviderId::new("openai").unwrap())
    );
    assert!(
        catalog
            .models()
            .get(&ModelId::new("gpt-4o").unwrap())
            .is_some()
    );
}

fn workflow(id: &str, routes: &[&str]) -> Workflow {
    Workflow::new(
        WorkflowId::new(id).unwrap(),
        routes
            .iter()
            .map(|route| RouteId::new(*route).unwrap())
            .collect(),
    )
    .unwrap()
}
