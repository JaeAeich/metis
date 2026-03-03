use std::path::PathBuf;
use std::sync::Arc;
use tempfile::TempDir;
use uuid::Uuid;
use engine::Engine;

#[tokio::test]
async fn test_dry_run_with_manual_test_config() {
    let temp_dir = TempDir::new().expect("Failed to create temp dir");
    
    let engine_config_path = PathBuf::from("manual_test/engine.yaml");
    let request_path = PathBuf::from("manual_test/request.json");
    
    assert!(engine_config_path.exists(), "engine.yaml should exist");
    assert!(request_path.exists(), "request.json should exist");
    
    let config_content = tokio::fs::read_to_string(&engine_config_path)
        .await
        .expect("Failed to read engine.yaml");
    
    let config: common::configs::FullEngineConfig = serde_yaml::from_str(&config_content)
        .expect("Failed to parse engine.yaml");
    
    let request_content = tokio::fs::read_to_string(&request_path)
        .await
        .expect("Failed to read request.json");
    
    let request: common::models::RunRequest = serde_json::from_str(&request_content)
        .expect("Failed to parse request.json");
    
    let mut config = config;
    let base_path = temp_dir.path().to_string_lossy().to_string();
    config.runs.workdir.base = base_path;
    
    let validator = common::validators::EngineRequestValidator::new(config.engine.clone());
    let validated_request = validator.validate(&request).expect("Validation failed");
    
    let runtime = engine::EngineRuntime::new(
        Arc::new(engine::NoopEngine),
        config.clone(),
        None,
        None,
        true,
    )
    .await
    .expect("Failed to create runtime");
    
    let run_id = Uuid::now_v7();
    let summary = runtime
        .run(run_id, "test_user".to_string(), validated_request)
        .await
        .expect("Run failed");
    
    assert_eq!(summary.run_id, run_id.to_string());
    assert!(summary.state.is_some());
    
    let workdir = format!("{}/test_user/{}", config.runs.workdir.base, run_id);
    assert!(
        std::path::Path::new(&workdir).exists(),
        "Workdir should exist: {}",
        workdir
    );
    
    if let Some(subdirs) = &config.runs.workdir.subdirs {
        for (name, subdir) in subdirs {
            let subdir_path = format!("{}/{}", workdir, subdir);
            assert!(
                std::path::Path::new(&subdir_path).exists(),
                "Subdir {} should exist: {}",
                name,
                subdir_path
            );
        }
    }
}

#[tokio::test]
async fn test_engine_trait_noop_implementation() {
    let engine = engine::NoopEngine;
    
    let results = engine.get_workflow_results().await.expect("get_workflow_results failed");
    assert!(results.is_none());
    
    let logs = engine.get_task_logs().await.expect("get_task_logs failed");
    assert!(logs.is_none());
}
