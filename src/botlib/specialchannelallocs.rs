use std::collections::HashMap;

pub fn create_special_allocation_from_str(
    special_allocations: &str,
) -> Result<HashMap<String, u32>, crate::Error> {
    let split = special_allocations.split(',').collect::<Vec<&str>>();

    if !split.is_empty() {
        let mut map = HashMap::new();

        for v in split {
            if v.is_empty() {
                continue;
            }

            let split = v.split('=').collect::<Vec<&str>>();

            if split.len() != 2 {
                return Err("Invalid special allocation format".into());
            }

            let channel_id = split[0].to_string();
            let number = split[1].parse::<u32>()?;

            map.insert(channel_id, number);
        }

        Ok(map)
    } else {
        Ok(HashMap::new())
    }
}
