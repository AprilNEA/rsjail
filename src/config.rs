use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JailConfig {
    pub name: String,
    pub hostname: Option<String>,
    pub chroot_dir: Option<String>,
    pub exec_bin: String,
    pub exec_args: Vec<String>,
    
    // Namespace configuration
    pub clone_newpid: bool,
    pub clone_newnet: bool,
    pub clone_newns: bool,
    pub clone_newuts: bool,
    pub clone_newipc: bool,
    pub clone_newuser: bool,
    
    // Resource limits
    pub rlimit_as: Option<u64>,      // Memory limit
    pub rlimit_cpu: Option<u64>,     // CPU time limit
    pub rlimit_nofile: Option<u64>,  // File descriptor limit
    
    // Mount points
    pub mounts: Vec<MountConfig>,
    
    // User configuration
    pub uid: Option<u32>,
    pub gid: Option<u32>,
    
    // Time limit
    pub time_limit: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountConfig {
    pub src: String,
    pub dst: String,
    pub fstype: Option<String>,
    pub is_bind: bool,
    pub rw: bool,
}

impl Default for JailConfig {
    fn default() -> Self {
        Self {
            name: "default".to_string(),
            hostname: None,
            chroot_dir: None,
            exec_bin: "/bin/sh".to_string(),
            exec_args: vec!["/bin/sh".to_string()],
            clone_newpid: true,
            clone_newnet: true,
            clone_newns: true,
            clone_newuts: true,
            clone_newipc: true,
            clone_newuser: true,
            rlimit_as: None,
            rlimit_cpu: None,
            rlimit_nofile: None,
            mounts: Vec::new(),
            uid: None,
            gid: None,
            time_limit: None,
        }
    }
}