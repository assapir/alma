use anyhow::Context;
use byte_unit::Byte;
use std::{
    fmt,
    fs::{self, DirEntry},
};

#[derive(Debug)]
pub struct Device {
    model: String,
    vendor: String,
    size: Byte,
    pub name: String,
}

impl fmt::Display for Device {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "{} {} ({})",
            self.vendor,
            self.model,
            self.size.get_appropriate_unit(true)
        )
    }
}

fn trimmed(source: String) -> String {
    String::from(source.trim_end())
}

fn is_removable_device(device: &DirEntry) -> Result<bool, anyhow::Error> {
    Ok(fs::read_to_string(device.path().join("removable"))
        .map(|v| v == "1\n")
        .context("Error querying storage devices")?)
}

fn get_model(device: &DirEntry) -> Result<String, anyhow::Error> {
    Ok(fs::read_to_string(device.path().join("device/model"))
        .map(trimmed)
        .context("Error querying storage devices")?)
}

pub fn get_storage_devices(allow_non_removable: bool) -> anyhow::Result<Vec<Device>> {
    Ok(fs::read_dir("/sys/block")
        .context("Error querying storage devices")?
        .filter_map(|entry| {
            let entry = entry.context("Error querying storage devices").ok()?;

            if !allow_non_removable && !is_removable_device(&entry).ok()? {
                return None;
            }

            let model = get_model(&entry).ok()?;
            if model == "CD-ROM" {
                return None;
            }

            Some(Device {
                name: entry
                    .path()
                    .file_name()
                    .expect("Could not get file name for dir entry /sys/block")
                    .to_string_lossy()
                    .into_owned(),
                model,
                vendor: fs::read_to_string(entry.path().join("device/vendor"))
                    .map(trimmed)
                    .context("Error querying storage devices")
                    .ok()?,
                size: Byte::from_bytes(
                    fs::read_to_string(entry.path().join("size"))
                        .context("Error querying storage devices")
                        .ok()?
                        .trim()
                        .parse::<u128>()
                        .context("Could not parse block size to unsigned integer (u128)")
                        .ok()?
                        * 512,
                ),
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanity() {
        let devices = get_storage_devices(false).expect("No devices");
        println!("{:?}", devices);
    }
}
