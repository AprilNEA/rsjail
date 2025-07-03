use anyhow::Result;
use rsjail::{JailConfig, MountConfig};
use serde_json;
use tempfile::TempDir;

#[test]
fn test_config_serialization() {
    let config = JailConfig {
        name: "test".to_string(),
        hostname: Some("test-host".to_string()),
        chroot_dir: Some("/tmp/test".to_string()),
        exec_bin: "/bin/sh".to_string(),
        exec_args: vec!["/bin/sh".to_string()],
        clone_newpid: true,
        clone_newnet: true,
        clone_newns: true,
        clone_newuts: true,
        clone_newipc: true,
        clone_newuser: true,
        rlimit_as: Some(1024 * 1024),
        rlimit_cpu: Some(10),
        rlimit_nofile: Some(64),
        mounts: vec![MountConfig {
            src: "/bin".to_string(),
            dst: "/bin".to_string(),
            fstype: None,
            is_bind: true,
            rw: false,
        }],
        uid: Some(1000),
        gid: Some(1000),
        time_limit: Some(30),
    };

    // Test serialization
    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("\"name\":\"test\""));

    // Test deserialization
    let deserialized: JailConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.name, "test");
    assert_eq!(deserialized.hostname, Some("test-host".to_string()));
}

#[test]
fn test_config_default() {
    let config = JailConfig::default();
    assert_eq!(config.name, "default");
    assert_eq!(config.exec_bin, "/bin/sh");
    assert!(config.clone_newpid);
    assert!(config.mounts.is_empty());
}

#[test]
fn test_config_validation() {
    let mut config = JailConfig::default();
    
    // Test empty exec file
    config.exec_bin = "".to_string();
    assert!(validate_config(&config).is_err());
    
    // Test invalid chroot directory
    config.exec_bin = "/bin/sh".to_string();
    config.chroot_dir = Some("/nonexistent/path".to_string());
    assert!(validate_config(&config).is_err());
    
    // Test valid config
    config.chroot_dir = Some("/tmp/test".to_string());
    assert!(validate_config(&config).is_ok());cargo test --lib
}

fn validate_config(config: &JailConfig) -> Result<()> {
    if config.exec_bin.is_empty() {
        return Err(anyhow::anyhow!("exec_bin cannot be empty"));
    }
    
    if let Some(chroot_dir) = &config.chroot_dir {
        if !std::path::Path::new(chroot_dir).exists() {
            return Err(anyhow::anyhow!("chroot_dir does not exist"));
        }
    }
    
    Ok(())
}
