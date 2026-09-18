use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
use serde::{Deserialize, Serialize};

pub const ORGB_MAGIC: &[u8; 4] = b"ORGB";
pub const ORGB_DEFAULT_PORT: u16 = 6742;

// Command IDs
#[allow(dead_code)]
pub const REQUEST_CONTROLLER_COUNT: u32 = 0;
pub const REQUEST_CONTROLLER_DATA: u32 = 1;
pub const SET_CLIENT_NAME: u32 = 50;
pub const RGBCONTROLLER_RESIZEZONE: u32 = 1000;
pub const RGBCONTROLLER_UPDATELEDS: u32 = 1050;
#[allow(dead_code)]
pub const RGBCONTROLLER_UPDATEZONELEDS: u32 = 1051;
pub const RGBCONTROLLER_SETCUSTOMMODE: u32 = 1100;
#[allow(dead_code)]
pub const RGBCONTROLLER_UPDATEMODE: u32 = 1101;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RgbColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl RgbColor {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub fn to_bytes(&self) -> [u8; 4] {
        [self.r, self.g, self.b, 0]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZoneInfo {
    pub id: u32,
    pub name: String,
    pub zone_type: i32,
    pub leds_min: u32,
    pub leds_max: u32,
    pub leds_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub id: u32,
    pub name: String,
    pub vendor: String,
    pub description: String,
    pub version: String,
    pub serial: String,
    pub location: String,
    pub led_count: u32,
    pub active_mode: i32,
    pub zones: Vec<ZoneInfo>,
}

pub struct OpenRgbClient {
    stream: TcpStream,
}

impl OpenRgbClient {
    pub fn connect(host: &str, port: u16) -> Result<Self, String> {
        let stream = TcpStream::connect((host, port))
            .map_err(|e| format!("Could not connect to OpenRGB at {}:{}: {}", host, port, e))?;
        stream.set_read_timeout(Some(Duration::from_secs(3))).ok();
        stream.set_write_timeout(Some(Duration::from_secs(3))).ok();

        let mut client = Self { stream };
        client.set_client_name("AuraSync-SmartRGB")?;
        Ok(client)
    }

    fn write_packet(&mut self, device_id: u32, command_id: u32, data: &[u8]) -> Result<(), String> {
        let mut header = Vec::with_capacity(16);
        header.extend_from_slice(ORGB_MAGIC);
        header.write_u32::<LittleEndian>(device_id).map_err(|e| e.to_string())?;
        header.write_u32::<LittleEndian>(command_id).map_err(|e| e.to_string())?;
        header.write_u32::<LittleEndian>(data.len() as u32).map_err(|e| e.to_string())?;

        self.stream.write_all(&header).map_err(|e| e.to_string())?;
        if !data.is_empty() {
            self.stream.write_all(data).map_err(|e| e.to_string())?;
        }
        self.stream.flush().map_err(|e| e.to_string())?;
        Ok(())
    }

    fn read_packet_header(&mut self) -> Result<(u32, u32, u32), String> {
        let mut header = [0u8; 16];
        self.stream.read_exact(&mut header).map_err(|e| format!("Header read failed: {}", e))?;

        if &header[0..4] != ORGB_MAGIC {
            return Err("Invalid OpenRGB magic header".into());
        }

        let mut cursor = &header[4..];
        let device_id = cursor.read_u32::<LittleEndian>().map_err(|e| e.to_string())?;
        let command_id = cursor.read_u32::<LittleEndian>().map_err(|e| e.to_string())?;
        let data_size = cursor.read_u32::<LittleEndian>().map_err(|e| e.to_string())?;

        Ok((device_id, command_id, data_size))
    }

    pub fn set_client_name(&mut self, name: &str) -> Result<(), String> {
        let mut data = name.as_bytes().to_vec();
        data.push(0); // Null terminated string
        self.write_packet(0, SET_CLIENT_NAME, &data)
    }

    pub fn get_controller_count(&mut self) -> Result<u32, String> {
        self.write_packet(0, REQUEST_CONTROLLER_COUNT, &[])?;
        let (_, _, size) = self.read_packet_header()?;
        if size < 4 {
            return Err("Unexpected packet size for controller count".into());
        }
        let mut count_buf = [0u8; 4];
        self.stream.read_exact(&mut count_buf).map_err(|e| e.to_string())?;
        let count = (&count_buf[..]).read_u32::<LittleEndian>().map_err(|e| e.to_string())?;
        Ok(count)
    }

    pub fn get_controller_data(&mut self, device_id: u32) -> Result<DeviceInfo, String> {
        // Request with protocol version 3 (OpenRGB 0.7+)
        let mut req_data = Vec::new();
        req_data.write_u32::<LittleEndian>(3).ok();
        self.write_packet(device_id, REQUEST_CONTROLLER_DATA, &req_data)?;

        let (_, _, size) = self.read_packet_header()?;
        let mut payload = vec![0u8; size as usize];
        self.stream.read_exact(&mut payload).map_err(|e| e.to_string())?;

        let mut cursor = &payload[..];
        // Read size prefix
        if cursor.len() >= 4 {
            let _total_size = cursor.read_u32::<LittleEndian>().unwrap_or(0);
        }
        let _dev_type = cursor.read_i32::<LittleEndian>().unwrap_or(0);
        let name = read_string(&mut cursor);
        let vendor = read_string(&mut cursor);
        let description = read_string(&mut cursor);
        let version = read_string(&mut cursor);
        let serial = read_string(&mut cursor);
        let location = read_string(&mut cursor);

        // Read modes count and active mode
        let modes_count = cursor.read_u16::<LittleEndian>().unwrap_or(0);
        let active_mode = cursor.read_i32::<LittleEndian>().unwrap_or(0);

        for _ in 0..modes_count {
            let _mode_name = read_string(&mut cursor);
            let _mode_val = cursor.read_i32::<LittleEndian>().unwrap_or(0);
            let _mode_flags = cursor.read_u32::<LittleEndian>().unwrap_or(0);
            let _speed_min = cursor.read_u32::<LittleEndian>().unwrap_or(0);
            let _speed_max = cursor.read_u32::<LittleEndian>().unwrap_or(0);
            // Protocol version >= 3 adds brightness_min and brightness_max
            let _brightness_min = cursor.read_u32::<LittleEndian>().unwrap_or(0);
            let _brightness_max = cursor.read_u32::<LittleEndian>().unwrap_or(0);
            let _colors_min = cursor.read_u32::<LittleEndian>().unwrap_or(0);
            let _colors_max = cursor.read_u32::<LittleEndian>().unwrap_or(0);
            let _speed = cursor.read_u32::<LittleEndian>().unwrap_or(0);
            // Protocol version >= 3 adds brightness
            let _brightness = cursor.read_u32::<LittleEndian>().unwrap_or(0);
            let _direction = cursor.read_u32::<LittleEndian>().unwrap_or(0);
            let _color_mode = cursor.read_u32::<LittleEndian>().unwrap_or(0);
            let colors_count = cursor.read_u16::<LittleEndian>().unwrap_or(0);
            for _ in 0..colors_count {
                let _ = cursor.read_u32::<LittleEndian>().unwrap_or(0);
            }
        }

        // Read zones
        let zones_count = cursor.read_u16::<LittleEndian>().unwrap_or(0);
        let mut zones = Vec::new();
        for z_idx in 0..zones_count {
            let zone_name = read_string(&mut cursor);
            let zone_type = cursor.read_i32::<LittleEndian>().unwrap_or(0);
            let leds_min = cursor.read_u32::<LittleEndian>().unwrap_or(0);
            let leds_max = cursor.read_u32::<LittleEndian>().unwrap_or(0);
            let leds_count = cursor.read_u32::<LittleEndian>().unwrap_or(0);
            let mat_len = cursor.read_u16::<LittleEndian>().unwrap_or(0);
            if mat_len > 0 && cursor.len() >= mat_len as usize {
                cursor = &cursor[mat_len as usize..];
            }
            zones.push(ZoneInfo {
                id: z_idx as u32,
                name: zone_name,
                zone_type,
                leds_min,
                leds_max,
                leds_count,
            });
        }

        // Read LEDs count and skip details
        let leds_count = cursor.read_u16::<LittleEndian>().unwrap_or(0);
        for _ in 0..leds_count {
            let _ = read_string(&mut cursor);
            let _ = cursor.read_u32::<LittleEndian>().unwrap_or(0);
        }

        // Read colors count and skip
        let colors_count = cursor.read_u16::<LittleEndian>().unwrap_or(0);
        let _ = colors_count;

        Ok(DeviceInfo {
            id: device_id,
            name,
            vendor,
            description,
            version,
            serial,
            location,
            led_count: leds_count as u32,
            active_mode,
            zones,
        })
    }

    pub fn resize_zone(&mut self, device_id: u32, zone_id: u32, size: u32) -> Result<(), String> {
        let mut data = Vec::with_capacity(8);
        data.write_i32::<LittleEndian>(zone_id as i32).map_err(|e| e.to_string())?;
        data.write_i32::<LittleEndian>(size as i32).map_err(|e| e.to_string())?;
        self.write_packet(device_id, RGBCONTROLLER_RESIZEZONE, &data)
    }

    pub fn update_leds(&mut self, device_id: u32, colors: &[RgbColor]) -> Result<(), String> {
        let mut data = Vec::new();
        // Reserve 4 bytes for total size (we write it at the end)
        data.write_u32::<LittleEndian>(0).ok();
        // Number of colors (u16)
        data.write_u16::<LittleEndian>(colors.len() as u16).map_err(|e| e.to_string())?;

        for c in colors {
            data.extend_from_slice(&c.to_bytes());
        }

        // Set total length at header
        let total_size = data.len() as u32;
        (&mut data[0..4]).write_u32::<LittleEndian>(total_size).ok();

        self.write_packet(device_id, RGBCONTROLLER_UPDATELEDS, &data)
    }

    #[allow(dead_code)]
    pub fn update_zone_leds(&mut self, device_id: u32, zone_id: u32, colors: &[RgbColor]) -> Result<(), String> {
        let mut data = Vec::new();
        // Reserve 4 bytes for total size
        data.write_u32::<LittleEndian>(0).ok();
        data.write_i32::<LittleEndian>(zone_id as i32).map_err(|e| e.to_string())?;
        data.write_u16::<LittleEndian>(colors.len() as u16).map_err(|e| e.to_string())?;

        for c in colors {
            data.extend_from_slice(&c.to_bytes());
        }

        let total_size = data.len() as u32;
        (&mut data[0..4]).write_u32::<LittleEndian>(total_size).ok();

        self.write_packet(device_id, RGBCONTROLLER_UPDATEZONELEDS, &data)
    }

    pub fn set_custom_mode(&mut self, device_id: u32) -> Result<(), String> {
        self.write_packet(device_id, RGBCONTROLLER_SETCUSTOMMODE, &[])
    }
}

fn read_string(cursor: &mut &[u8]) -> String {
    if cursor.len() < 2 {
        return String::new();
    }
    let length = cursor.read_u16::<LittleEndian>().unwrap_or(0) as usize;
    if length == 0 || cursor.len() < length {
        return String::new();
    }
    let str_bytes = &cursor[..length.saturating_sub(1)]; // Remove null terminator if present
    let result = String::from_utf8_lossy(str_bytes).to_string();
    *cursor = &cursor[length..];
    result
}
