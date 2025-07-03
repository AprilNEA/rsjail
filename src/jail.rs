use crate::config::{JailConfig, MountConfig};
use anyhow::Result;
#[cfg(target_os = "linux")]
use nix::mount::{mount, MsFlags};
use nix::sched::{unshare, CloneFlags};
use nix::sys::resource::{setrlimit, Resource};
use nix::sys::wait::{waitpid, WaitStatus};
use nix::unistd::{
    chroot, execve, fork, getgid, getpid, getuid, setgid, sethostname, setuid, ForkResult, Gid,
    Pid, Uid,
};
use std::ffi::CString;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;

pub struct Jail {
    config: JailConfig,
}

impl Jail {
    pub fn new(config: JailConfig) -> Self {
        Self { config }
    }

    pub fn run(&self) -> Result<()> {
        // Create Namespace
        self.create_namespaces()?;
        
        // fork child process
        match unsafe { fork() }? {
            ForkResult::Parent { child } => {
                // Parent process wait for child process
                self.wait_for_child(child)?;
            }
            ForkResult::Child => {
                // Child process setup environment and execute program
                if let Err(e) = self.setup_child_environment() {
                    eprintln!("Child setup failed: {}", e);
                    std::process::exit(1);
                }
            }
        }
        
        Ok(())
    }

    fn create_namespaces(&self) -> Result<()> {
        let mut flags = CloneFlags::empty();
        
        if self.config.clone_newpid {
            flags |= CloneFlags::CLONE_NEWPID;
        }
        if self.config.clone_newnet {
            flags |= CloneFlags::CLONE_NEWNET;
        }
        if self.config.clone_newns {
            flags |= CloneFlags::CLONE_NEWNS;
        }
        if self.config.clone_newuts {
            flags |= CloneFlags::CLONE_NEWUTS;
        }
        if self.config.clone_newipc {
            flags |= CloneFlags::CLONE_NEWIPC;
        }
        if self.config.clone_newuser {
            flags |= CloneFlags::CLONE_NEWUSER;
        }

        unshare(flags)?;
        Ok(())
    }

    fn setup_child_environment(&self) -> Result<()> {
        // Setup user namespace mapping
        if self.config.clone_newuser {
            self.setup_uid_gid_mapping()?;
        }

        // Setup hostname
        if let Some(hostname) = &self.config.hostname {
            sethostname(hostname)?;
        }

        // Setup filesystem
        if let Some(chroot_dir) = &self.config.chroot_dir {
            self.setup_filesystem(chroot_dir)?;
        }

        // Setup user permissions
        self.setup_user_permissions()?;

        // Setup resource limits
        self.setup_resource_limits()?;

        // Execute target program
        self.exec_target_program()?;

        Ok(())
    }

    fn setup_uid_gid_mapping(&self) -> Result<()> {
        let pid = getpid();
        
        // Setup UID mapping
        let uid_map = format!("0 {} 1", getuid());
        let uid_map_path = format!("/proc/{}/uid_map", pid);
        let mut uid_map_file = OpenOptions::new().write(true).open(&uid_map_path)?;
        uid_map_file.write_all(uid_map.as_bytes())?;
        
        // Disable setgroups
        let setgroups_path = format!("/proc/{}/setgroups", pid);
        let mut setgroups_file = OpenOptions::new().write(true).open(&setgroups_path)?;
        setgroups_file.write_all(b"deny")?;
        
        // Setup GID mapping
        let gid_map = format!("0 {} 1", getgid());
        let gid_map_path = format!("/proc/{}/gid_map", pid);
        let mut gid_map_file = OpenOptions::new().write(true).open(&gid_map_path)?;
        gid_map_file.write_all(gid_map.as_bytes())?;
        
        Ok(())
    }

    fn setup_filesystem(&self, chroot_dir: &str) -> Result<()> {
        // Create basic directory structure
        self.create_jail_directories(chroot_dir)?;
        
        // Setup mount points
        for mount_config in &self.config.mounts {
            self.setup_mount(chroot_dir, mount_config)?;
        }
        
        // Switch root directory
        chroot(chroot_dir)?;
        std::env::set_current_dir("/")?;
        
        Ok(())
    }

    fn create_jail_directories(&self, chroot_dir: &str) -> Result<()> {
        let base_path = Path::new(chroot_dir);
        
        // Create basic directory structure
        if !base_path.exists() {
            fs::create_dir_all(base_path)?;
        }
        
        let dirs = ["bin", "lib", "lib64", "usr", "etc", "tmp", "proc", "dev", "sys"];
        for dir in dirs {
            let dir_path = base_path.join(dir);
            if !dir_path.exists() {
                fs::create_dir_all(&dir_path)?;
            }
        }
        
        Ok(())
    }

    fn setup_mount(&self, chroot_dir: &str, mount_config: &MountConfig) -> Result<()> {
        let target = format!("{}{}", chroot_dir, mount_config.dst);
        
        // Ensure target directory exists
        if let Some(parent) = Path::new(&target).parent() {
            fs::create_dir_all(parent)?;
        }
        
        // If target is a file, create an empty file
        if Path::new(&mount_config.src).is_file() {
            fs::File::create(&target)?;
        } else if !Path::new(&target).exists() {
            fs::create_dir_all(&target)?;
        }
        
        let mut flags = MsFlags::empty();
        if mount_config.is_bind {
            flags |= MsFlags::MS_BIND;
        }
        if !mount_config.rw {
            flags |= MsFlags::MS_RDONLY;
        }
        
        mount(
            Some(mount_config.src.as_str()),
            target.as_str(),
            mount_config.fstype.as_deref(),
            flags,
            None::<&str>,
        )?;
        
        Ok(())
    }

    fn setup_user_permissions(&self) -> Result<()> {
        if let Some(gid) = self.config.gid {
            setgid(Gid::from_raw(gid))?;
        }
        
        if let Some(uid) = self.config.uid {
            setuid(Uid::from_raw(uid))?;
        }
        
        Ok(())
    }

    fn setup_resource_limits(&self) -> Result<()> {
        if let Some(mem_limit) = self.config.rlimit_as {
            setrlimit(Resource::RLIMIT_AS, mem_limit, mem_limit)?;
        }
        
        if let Some(cpu_limit) = self.config.rlimit_cpu {
            setrlimit(Resource::RLIMIT_CPU, cpu_limit, cpu_limit)?;
        }
        
        if let Some(nofile_limit) = self.config.rlimit_nofile {
            setrlimit(Resource::RLIMIT_NOFILE, nofile_limit, nofile_limit)?;
        }
        
        Ok(())
    }

    fn exec_target_program(&self) -> Result<()> {
        let program = CString::new(self.config.exec_bin.clone())?;
        
        let args: Result<Vec<CString>, _> = self
            .config
            .exec_args
            .iter()
            .map(|arg| CString::new(arg.clone()))
            .collect();
        let args = args?;
        
        let env = vec![
            CString::new("PATH=/bin:/usr/bin:/sbin:/usr/sbin")?,
            CString::new("HOME=/")?,
            CString::new("USER=jail")?,
        ];
        
        execve(&program, &args, &env)?;
        Ok(())
    }

    fn wait_for_child(&self, child: Pid) -> Result<()> {
        match waitpid(child, None)? {
            WaitStatus::Exited(pid, code) => {
                println!("Child {} exited with code {}", pid, code);
            }
            WaitStatus::Signaled(pid, signal, _) => {
                println!("Child {} killed by signal {:?}", pid, signal);
            }
            _ => {
                println!("Child process status changed");
            }
        }
        Ok(())
    }
}
