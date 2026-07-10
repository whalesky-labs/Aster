use calamine::Data;

pub fn to_text(data: &Data) -> String {
    match data {
        Data::String(value) => value.trim().to_string(),
        Data::Float(value) => trim_number(*value),
        Data::Int(value) => value.to_string(),
        Data::Bool(value) => value.to_string(),
        Data::DateTime(value) => {
            let (year, month, day, hour, minute, second, _) = value.to_ymd_hms_milli();
            format!("{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02}")
        }
        _ => String::new(),
    }
}

pub fn to_number(data: &Data) -> Option<f64> {
    match data {
        Data::Float(value) => Some(*value),
        Data::Int(value) => Some(*value as f64),
        Data::String(value) => parse_number_text(value),
        _ => None,
    }
    .filter(|value| value.is_finite())
}

pub fn is_empty(data: &Data) -> bool {
    match data {
        Data::Empty => true,
        Data::String(value) => value.trim().is_empty(),
        _ => false,
    }
}

fn parse_number_text(value: &str) -> Option<f64> {
    let cleaned = value.trim().replace([',', '，', '￥'], "");
    if cleaned.is_empty() || cleaned == "-" {
        None
    } else {
        cleaned.parse::<f64>().ok()
    }
}

fn trim_number(value: f64) -> String {
    if value.fract().abs() < f64::EPSILON {
        format!("{}", value as i64)
    } else {
        value.to_string()
    }
}
