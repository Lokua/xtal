use wgpu::util::DeviceExt;
use crate::warn_once;

pub struct UniformBanks {
    data: Vec<[f32; 4]>,
    buffer: wgpu::Buffer,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

impl UniformBanks {
    pub fn new(device: &wgpu::Device, banks: usize) -> Self {
        assert!(banks > 0, "uniform bank count must be > 0");

        let data = vec![[0.0; 4]; banks];

        let bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("xtal2-uniform-banks-layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX
                        | wgpu::ShaderStages::FRAGMENT
                        | wgpu::ShaderStages::COMPUTE,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: wgpu::BufferSize::new(
                            (data.len() * std::mem::size_of::<[f32; 4]>())
                                as u64,
                        ),
                    },
                    count: None,
                }],
            });

        let buffer =
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("xtal2-uniform-banks-buffer"),
                contents: bytemuck::cast_slice(&data),
                usage: wgpu::BufferUsages::UNIFORM
                    | wgpu::BufferUsages::COPY_DST,
            });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("xtal2-uniform-banks-bind-group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: buffer.as_entire_binding(),
            }],
        });

        Self {
            data,
            buffer,
            bind_group_layout,
            bind_group,
        }
    }

    pub fn set_resolution(&mut self, w: f32, h: f32) {
        self.data[0][0] = w;
        self.data[0][1] = h;
    }

    pub fn set_beats(&mut self, beats: f32) {
        self.data[0][2] = beats;
    }

    pub fn set(&mut self, bank: &str, value: f32) -> Result<(), String> {
        let (bank_idx, component_idx) =
            parse_bank_component(bank).map_err(|message| {
                format!("invalid bank '{}': {}", bank, message)
            })?;

        if bank_idx >= self.data.len() {
            return Err(format!(
                "bank index out of bounds for '{}': {} >= {}",
                bank,
                bank_idx,
                self.data.len()
            ));
        }

        self.data[bank_idx][component_idx] = value;
        Ok(())
    }

    pub fn upload(&self, queue: &wgpu::Queue) {
        queue.write_buffer(&self.buffer, 0, bytemuck::cast_slice(&self.data));
    }

    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

fn parse_bank_component(input: &str) -> Result<(usize, usize), &'static str> {
    if input.len() != 2 {
        return Err("expected exactly two chars like 'ax'");
    }

    let mut chars = input.chars();
    let bank_char = chars.next().ok_or("missing bank char")?;
    let component_char = chars.next().ok_or("missing component char")?;

    if !bank_char.is_ascii_lowercase() {
        return Err("bank must be lowercase a-z");
    }

    let bank_idx = (bank_char as u8 - b'a') as usize;
    let component_idx = match component_char {
        'x' | 'X' => 0,
        'y' | 'Y' => 1,
        'z' | 'Z' => 2,
        'w' | 'W' => 3,
        '1' => {
            warn_once!(
                "Deprecated uniform alias '{}': use '{}x' instead",
                input,
                bank_char
            );
            0
        }
        '2' => {
            warn_once!(
                "Deprecated uniform alias '{}': use '{}y' instead",
                input,
                bank_char
            );
            1
        }
        '3' => {
            warn_once!(
                "Deprecated uniform alias '{}': use '{}z' instead",
                input,
                bank_char
            );
            2
        }
        '4' => {
            warn_once!(
                "Deprecated uniform alias '{}': use '{}w' instead",
                input,
                bank_char
            );
            3
        }
        _ => return Err("component must be x/y/z/w (legacy 1..4 allowed)"),
    };

    Ok((bank_idx, component_idx))
}

#[cfg(test)]
mod tests {
    use super::parse_bank_component;

    #[test]
    fn parses_letter_components() {
        assert_eq!(parse_bank_component("ax").unwrap(), (0, 0));
        assert_eq!(parse_bank_component("ay").unwrap(), (0, 1));
        assert_eq!(parse_bank_component("az").unwrap(), (0, 2));
        assert_eq!(parse_bank_component("aw").unwrap(), (0, 3));
        assert_eq!(parse_bank_component("bw").unwrap(), (1, 3));
    }

    #[test]
    fn keeps_legacy_numeric_aliases() {
        assert_eq!(parse_bank_component("a1").unwrap(), (0, 0));
        assert_eq!(parse_bank_component("a2").unwrap(), (0, 1));
        assert_eq!(parse_bank_component("a3").unwrap(), (0, 2));
        assert_eq!(parse_bank_component("a4").unwrap(), (0, 3));
    }

    #[test]
    fn rejects_invalid_components() {
        assert!(parse_bank_component("a0").is_err());
        assert!(parse_bank_component("av").is_err());
        assert!(parse_bank_component("A1").is_err());
    }
}
