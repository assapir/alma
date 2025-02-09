use super::markers::{BlockDevice, Origin};
use super::partition::Partition;
use anyhow::{anyhow, Context};
use log::debug;
use nix::mount::umount;
use std::fs::{self, read_to_string};
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct StorageDevice<'a> {
    name: String,
    path: PathBuf,
    origin: PhantomData<&'a dyn Origin>,
    mount_config: Vec<MountConfig>,
}

#[derive(Debug)]
pub struct MountConfig {
    pub mount_point: PathBuf,
}

impl<'a> StorageDevice<'a> {
    pub fn from_path(path: &'a Path, allow_non_removable: bool) -> anyhow::Result<Self> {
        debug!("path: {:?}", path);
        let path = path
            .canonicalize()
            .context("Error querying information about the block device")?;
        let device_name = path
            .file_name()
            .and_then(std::ffi::OsStr::to_str)
            .map(String::from)
            .ok_or_else(|| anyhow!("Invalid device name: {}", path.display()))?;

        debug!("real path: {:?}, device name: {:?}", path, device_name);

        let path_as_str = path.to_str().expect("Unable to get the path as &str ");
        let mount_config = Self::get_mount_point(path_as_str)?;

        let _self = Self {
            name: device_name,
            path,
            mount_config,
            origin: PhantomData,
        };

        // If we only allow removable/loop devices, and the device is neither removable or a loop
        // device then throw a DangerousDevice error
        if !(allow_non_removable || _self.is_removable_device()? || _self.is_loop_device()) {
            return Err(anyhow!(
                "The given block device is neither removable nor a loop device: {}",
                _self.name
            ));
        }

        Ok(_self)
    }

    fn get_mount_point(path: &str) -> anyhow::Result<Vec<MountConfig>> {
        let mounts = fs::read_to_string("/proc/mounts").context("Unable to read /proc/mounts")?;
        let mount_line: Vec<MountConfig> = mounts
            .lines()
            .filter(|line| line.starts_with(path))
            .map(|mount_line| {
                let mut mount_point = mount_line.split_ascii_whitespace();
                let path = PathBuf::from(mount_point.nth(1).unwrap());
                MountConfig { mount_point: path }
            })
            .collect();
        Ok(mount_line)
    }

    fn sys_path(&self) -> PathBuf {
        let mut path = PathBuf::from("/sys/block");
        path.push(self.name.clone());
        path
    }

    fn is_removable_device(&self) -> anyhow::Result<bool> {
        let mut path = self.sys_path();
        path.push("removable");

        debug!("Reading: {:?}", path);
        let result =
            read_to_string(&path).context("Error querying information about the block device")?;
        debug!("{:?} -> {}", path, result);

        Ok(result == "1\n")
    }

    fn is_loop_device(&self) -> bool {
        let mut path = self.sys_path();
        path.push("loop");
        path.exists()
    }

    pub fn get_partition(&self, index: u8) -> anyhow::Result<Partition> {
        let name = if self
            .name
            .chars()
            .next_back()
            .expect("Storage device name is empty")
            .is_ascii_digit()
        {
            format!("{}p{}", self.name, index)
        } else {
            format!("{}{}", self.name, index)
        };
        let mut path = PathBuf::from("/dev");
        path.push(name);

        debug!("Partition {} for {} is in {:?}", index, self.name, path);
        if !path.exists() {
            return Err(anyhow!("Partition {} does not exist", index));
        }
        Ok(Partition::new::<Self>(path))
    }

    pub fn umount_if_needed(&mut self) {
        for config in &self.mount_config {
            debug!("Unmounting {:?}", config.mount_point);
            let _ = umount(&config.mount_point);
        }
        self.mount_config = vec![]
    }
}

impl<'a> BlockDevice for StorageDevice<'a> {
    fn path(&self) -> &Path {
        &self.path
    }
}

impl<'a> Origin for StorageDevice<'a> {}
