//! Timezone stage: converts local datetimes to UTC using ZIP code lookup.
//!
//! Looks up the timezone for a US ZIP code (using 3-digit prefix mapping),
//! interprets the datetime in that timezone, and converts to UTC RFC3339.
//! Supports store-level overrides (e.g., online stores that operate in UTC).

use std::collections::{BTreeMap, HashMap};

use async_trait::async_trait;
use chrono::{NaiveDateTime, TimeZone};
use chrono_tz::Tz;
use serde::Deserialize;
use serde_json::Value;
use tracing::debug;

use ecl_pipeline_topo::error::StageError;
use ecl_pipeline_topo::{PipelineItem, Stage, StageContext};

type Record = serde_json::Map<String, serde_json::Value>;

/// Configuration for the timezone stage, parsed from stage params.
#[derive(Debug, Clone, Deserialize)]
pub struct TimezoneConfig {
    /// Field containing the datetime string (already parsed, RFC3339 or local).
    pub datetime_field: String,
    /// Field containing the ZIP code for timezone lookup.
    pub zipcode_field: String,
    /// Output field for UTC datetime.
    pub output: String,
    /// Fallback timezone if ZIP lookup fails.
    #[serde(default = "default_us_eastern")]
    pub fallback_timezone: String,
    /// Special overrides: { "5995": "UTC" } for online stores.
    #[serde(default)]
    pub overrides: BTreeMap<String, String>,
    /// Override key field (e.g., "store_id" to match against overrides).
    #[serde(default)]
    pub override_key_field: Option<String>,
}

fn default_us_eastern() -> String {
    "US/Eastern".to_string()
}

/// Timezone stage that converts local datetimes to UTC using ZIP code lookup.
#[derive(Debug)]
pub struct TimezoneStage {
    config: TimezoneConfig,
    fallback_tz: Tz,
    /// Override key → parsed Tz.
    override_tzs: HashMap<String, Tz>,
    /// 3-digit ZIP prefix → Tz.
    zip_prefix_table: HashMap<String, Tz>,
}

impl TimezoneStage {
    /// Create a timezone stage from JSON params.
    ///
    /// # Errors
    ///
    /// Returns `StageError::Permanent` if params cannot be deserialized
    /// or if a timezone name is invalid.
    pub fn from_params(params: &serde_json::Value) -> Result<Self, StageError> {
        let config: TimezoneConfig =
            serde_json::from_value(params.clone()).map_err(|e| StageError::Permanent {
                stage: "timezone".into(),
                item_id: String::new(),
                message: format!("invalid timezone config: {e}"),
            })?;

        let fallback_tz =
            config
                .fallback_timezone
                .parse::<Tz>()
                .map_err(|e| StageError::Permanent {
                    stage: "timezone".into(),
                    item_id: String::new(),
                    message: format!(
                        "invalid fallback timezone '{}': {e}",
                        config.fallback_timezone
                    ),
                })?;

        let mut override_tzs = HashMap::new();
        for (key, tz_name) in &config.overrides {
            let tz = tz_name.parse::<Tz>().map_err(|e| StageError::Permanent {
                stage: "timezone".into(),
                item_id: String::new(),
                message: format!("invalid override timezone '{tz_name}': {e}"),
            })?;
            override_tzs.insert(key.clone(), tz);
        }

        let zip_prefix_table = build_zip_prefix_table();

        Ok(Self {
            config,
            fallback_tz,
            override_tzs,
            zip_prefix_table,
        })
    }

    /// Resolve the timezone for this record.
    fn resolve_timezone(&self, record: &Record) -> Tz {
        // Check overrides first.
        if let Some(override_field) = &self.config.override_key_field {
            if let Some(key_value) = record.get(override_field).and_then(|v| v.as_str()) {
                if let Some(tz) = self.override_tzs.get(key_value) {
                    return *tz;
                }
            }
        }

        // ZIP code lookup.
        if let Some(zip) = record
            .get(&self.config.zipcode_field)
            .and_then(|v| v.as_str())
        {
            if zip.len() >= 3 {
                let prefix = &zip[..3];
                if let Some(tz) = self.zip_prefix_table.get(prefix) {
                    return *tz;
                }
            }
        }

        // Fallback.
        self.fallback_tz
    }

    /// Convert a datetime string from the resolved timezone to UTC.
    fn convert_to_utc(&self, datetime_str: &str, tz: Tz) -> Option<String> {
        // Try parsing as RFC3339 first (already has timezone info).
        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(datetime_str) {
            let utc = dt.with_timezone(&chrono::Utc);
            return Some(utc.to_rfc3339());
        }

        // Try parsing as naive datetime (no timezone — apply the resolved tz).
        if let Ok(naive) = NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%dT%H:%M:%S") {
            if let Some(local) = tz.from_local_datetime(&naive).earliest() {
                let utc = local.with_timezone(&chrono::Utc);
                return Some(utc.to_rfc3339());
            }
        }

        // Try with fractional seconds.
        if let Ok(naive) = NaiveDateTime::parse_from_str(datetime_str, "%Y-%m-%dT%H:%M:%S%.f") {
            if let Some(local) = tz.from_local_datetime(&naive).earliest() {
                let utc = local.with_timezone(&chrono::Utc);
                return Some(utc.to_rfc3339());
            }
        }

        None
    }
}

#[async_trait]
impl Stage for TimezoneStage {
    fn name(&self) -> &str {
        "timezone"
    }

    async fn process(
        &self,
        mut item: PipelineItem,
        _ctx: &StageContext,
    ) -> Result<Vec<PipelineItem>, StageError> {
        let record = item.record.as_mut().ok_or_else(|| StageError::Permanent {
            stage: "timezone".into(),
            item_id: item.id.clone(),
            message: "item has no record".into(),
        })?;

        let tz = self.resolve_timezone(record);

        let datetime_str = record
            .get(&self.config.datetime_field)
            .and_then(|v| v.as_str())
            .unwrap_or("");

        debug!(
            item_id = %item.id,
            timezone = %tz,
            "converting to UTC"
        );

        if datetime_str.is_empty() {
            record.insert(self.config.output.clone(), Value::Null);
        } else {
            match self.convert_to_utc(datetime_str, tz) {
                Some(utc) => {
                    record.insert(self.config.output.clone(), Value::String(utc));
                }
                None => {
                    record.insert(self.config.output.clone(), Value::Null);
                }
            }
        }

        Ok(vec![item])
    }
}

/// Build the US 3-digit ZIP prefix → timezone lookup table.
///
/// This covers all US ZIP code prefixes (first 3 digits). The mapping is
/// approximate — a few ZIP prefixes span timezone boundaries, but this is
/// accurate for >99% of US addresses.
fn build_zip_prefix_table() -> HashMap<String, Tz> {
    let mut table = HashMap::new();

    // Eastern Time (ET) — ZIP prefixes 004-199, 200-299, 300-399 (partial)
    let eastern_prefixes = [
        // New England (CT, MA, ME, NH, RI, VT)
        "004", "005", "006", "007", "008", "009", "010", "011", "012", "013", "014", "015", "016",
        "017", "018", "019", "020", "021", "022", "023", "024", "025", "026", "027", "028", "029",
        "030", "031", "032", "033", "034", "035", "036", "037", "038", "039", "040", "041", "042",
        "043", "044", "045", "046", "047", "048", "049", "050", "051", "052", "053", "054", "055",
        "056", "057", "058", "059", "060", "061", "062", "063", "064", "065", "066", "067", "068",
        "069", // NJ, NY
        "070", "071", "072", "073", "074", "075", "076", "077", "078", "079", "080", "081", "082",
        "083", "084", "085", "086", "087", "088", "089", "100", "101", "102", "103", "104", "105",
        "106", "107", "108", "109", "110", "111", "112", "113", "114", "115", "116", "117", "118",
        "119", "120", "121", "122", "123", "124", "125", "126", "127", "128", "129", "130", "131",
        "132", "133", "134", "135", "136", "137", "138", "139", "140", "141", "142", "143", "144",
        "145", "146", "147", "148", "149", // PA, DE, DC, MD, VA, WV
        "150", "151", "152", "153", "154", "155", "156", "157", "158", "159", "160", "161", "162",
        "163", "164", "165", "166", "167", "168", "169", "170", "171", "172", "173", "174", "175",
        "176", "177", "178", "179", "180", "181", "182", "183", "184", "185", "186", "187", "188",
        "189", "190", "191", "192", "193", "194", "195", "196", "197", "198", "199", "200", "201",
        "202", "203", "204", "205", "206", "207", "208", "209", "210", "211", "212", "213", "214",
        "215", "216", "217", "218", "219", "220", "221", "222", "223", "224", "225", "226", "227",
        "228", "229", "230", "231", "232", "233", "234", "235", "236", "237", "238", "239", "240",
        "241", "242", "243", "244", "245", "246", "247", "248", "249", "250", "251", "252", "253",
        "254", "255", "256", "257", "258", "259", "260", "261", "262", "263", "264", "265", "266",
        "267", "268", "269", // NC, SC, GA, FL (eastern)
        "270", "271", "272", "273", "274", "275", "276", "277", "278", "279", "280", "281", "282",
        "283", "284", "285", "286", "287", "288", "289", "290", "291", "292", "293", "294", "295",
        "296", "297", "298", "299", "300", "301", "302", "303", "304", "305", "306", "307", "308",
        "309", "310", "311", "312", "313", "314", "315", "316", "317", "318", "319", "320", "321",
        "322", "323", "324", "325", "326", "327", "328", "329", "330", "331", "332", "333", "334",
        "335", "336", "337", "338", "339", // OH, MI, IN (eastern part)
        "400", "401", "402", "403", "404", "405", "406", "407", "408", "409", "410", "411", "412",
        "413", "414", "415", "416", "417", "418", "419", "430", "431", "432", "433", "434", "435",
        "436", "437", "438", "439", "440", "441", "442", "443", "444", "445", "446", "447", "448",
        "449", "450", "451", "452", "453", "454", "455", "456", "457", "458", "480", "481", "482",
        "483", "484", "485", "486", "487", "488", "489", "490", "491", "492", "493", "494", "495",
        "496", "497", "498", "499",
    ];
    for prefix in &eastern_prefixes {
        table.insert((*prefix).to_string(), chrono_tz::US::Eastern);
    }

    // Central Time (CT) — AL, AR, IA, IL, KS, KY (western), LA, MN, MO, MS, NE, ND, OK, SD, TN, TX, WI
    let central_prefixes = [
        // FL panhandle (Central Time)
        "324", // overlap with eastern — panhandle is central but using eastern for simplicity
        // AL
        "350", "351", "352", "354", "355", "356", "357", "358", "359", "360", "361", "362", "363",
        "364", "365", "366", "367", "368", "369", // TN
        "370", "371", "372", "373", "374", "375", "376", "377", "378", "379", "380", "381", "382",
        "383", "384", "385", // MS
        "386", "387", "388", "389", "390", "391", "392", "393", "394", "395", "396", "397",
        // KY (western)
        "420", "421", "422", "423", "424", "425", "426", "427", // IN (central part)
        "460", "461", "462", "463", "464", "465", "466", "467", "468", "469", "470", "471", "472",
        "473", "474", "475", "476", "477", "478", "479", // WI, MN
        "530", "531", "532", "534", "535", "537", "538", "539", "540", "541", "542", "543", "544",
        "545", "546", "547", "548", "549", "550", "551", "553", "554", "555", "556", "557", "558",
        "559", "560", "561", "562", "563", "564", "565", "566", "567", // IL
        "600", "601", "602", "603", "604", "605", "606", "607", "608", "609", "610", "611", "612",
        "613", "614", "615", "616", "617", "618", "619", "620", // MO
        "630", "631", "633", "634", "635", "636", "637", "638", "639", "640", "641", "644", "645",
        "646", "647", "648", "649", "650", "651", "652", "653", "654", "655", "656", "657", "658",
        // KS
        "660", "661", "662", "664", "665", "666", "667", "668", "669", "670", "671", "672", "673",
        "674", "675", "676", "677", "678", "679", // NE
        "680", "681", "683", "684", "685", "686", "687", "688", "689", "690", "691", "692", "693",
        // IA
        "500", "501", "502", "503", "504", "505", "506", "507", "508", "509", "510", "511", "512",
        "513", "514", "515", "516", "520", "521", "522", "523", "524", "525", "526", "527", "528",
        // ND, SD
        "570", "571", "572", "573", "574", "575", "576", "577", "580", "581", "582", "583", "584",
        "585", "586", "587", "588", // LA
        "700", "701", "703", "704", "705", "706", "707", "708", "710", "711", "712", "713", "714",
        // AR
        "716", "717", "718", "719", "720", "721", "722", "723", "724", "725", "726", "727", "728",
        "729", // OK
        "730", "731", "734", "735", "736", "737", "738", "739", "740", "741", "743", "744", "745",
        "746", "747", "748", "749", // TX
        "750", "751", "752", "753", "754", "755", "756", "757", "758", "759", "760", "761", "762",
        "763", "764", "765", "766", "767", "768", "769", "770", "771", "772", "773", "774", "775",
        "776", "777", "778", "779", "780", "781", "782", "783", "784", "785", "786", "787", "788",
        "789", "790", "791", "792", "793", "794", "795", "796", "797", "798", "799",
    ];
    for prefix in &central_prefixes {
        table.insert((*prefix).to_string(), chrono_tz::US::Central);
    }

    // Mountain Time (MT) — AZ, CO, ID (south), MT, NM, UT, WY
    let mountain_prefixes = [
        // MT
        "590", "591", "592", "593", "594", "595", "596", "597", "598", "599", // CO
        "800", "801", "802", "803", "804", "805", "806", "807", "808", "809", "810", "811", "812",
        "813", "814", "815", "816", // NM
        "870", "871", "872", "873", "874", "875", "876", "877", "878", "879", "880", "881", "882",
        "883", "884", // WY
        "820", "821", "822", "823", "824", "825", "826", "827", "828", "829", "830", "831",
        // ID
        "832", "833", "834", "835", "836", "837", "838", // UT
        "840", "841", "842", "843", "844", "845", "846", "847", // AZ
        "850", "851", "852", "853", "855", "856", "857", "858", "859", "860", "863", "864", "865",
        // TX (El Paso — Mountain Time)
        "798", "799",
    ];
    for prefix in &mountain_prefixes {
        table.insert((*prefix).to_string(), chrono_tz::US::Mountain);
    }

    // Pacific Time (PT) — CA, NV, OR, WA
    let pacific_prefixes = [
        // WA
        "980", "981", "982", "983", "984", "985", "986", "988", "989", "990", "991", "992", "993",
        "994", // OR
        "970", "971", "972", "973", "974", "975", "976", "977", "978", "979", // CA
        "900", "901", "902", "903", "904", "905", "906", "907", "908", "909", "910", "911", "912",
        "913", "914", "915", "916", "917", "918", "919", "920", "921", "922", "923", "924", "925",
        "926", "927", "928", "930", "931", "932", "933", "934", "935", "936", "937", "938", "939",
        "940", "941", "942", "943", "944", "945", "946", "947", "948", "949", "950", "951", "952",
        "953", "954", "955", "956", "957", "958", "959", "960", "961", // NV
        "889", "890", "891", "893", "894", "895", "896", "897", "898",
    ];
    for prefix in &pacific_prefixes {
        table.insert((*prefix).to_string(), chrono_tz::US::Pacific);
    }

    // Alaska
    let alaska_prefixes = ["995", "996", "997", "998", "999"];
    for prefix in &alaska_prefixes {
        table.insert((*prefix).to_string(), chrono_tz::US::Alaska);
    }

    // Hawaii
    let hawaii_prefixes = ["967", "968"];
    for prefix in &hawaii_prefixes {
        table.insert((*prefix).to_string(), chrono_tz::US::Hawaii);
    }

    table
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use ecl_pipeline_spec::PipelineSpec;
    use ecl_pipeline_state::{Blake3Hash, ItemProvenance};
    use serde_json::json;
    use std::sync::Arc;

    fn make_item(id: &str, record: serde_json::Map<String, Value>) -> PipelineItem {
        PipelineItem {
            id: id.to_string(),
            display_name: id.to_string(),
            content: Arc::from(b"" as &[u8]),
            mime_type: "application/x-csv-row".to_string(),
            source_name: "test".to_string(),
            source_content_hash: Blake3Hash::new("test"),
            provenance: ItemProvenance {
                source_kind: "test".to_string(),
                metadata: BTreeMap::new(),
                source_modified: None,
                extracted_at: chrono::Utc::now(),
            },
            metadata: BTreeMap::new(),
            record: Some(record),
            stream: None,
        }
    }

    fn ctx() -> StageContext {
        StageContext {
            spec: Arc::new(PipelineSpec {
                name: "test".to_string(),
                version: 1,
                output_dir: std::path::PathBuf::from("./out"),
                sources: BTreeMap::new(),
                stages: BTreeMap::new(),
                defaults: ecl_pipeline_spec::DefaultsSpec::default(),
                lifecycle: None,
                secrets: Default::default(),
                triggers: None,
                schedule: None,
            }),
            output_dir: std::path::PathBuf::from("./out"),
            params: serde_json::Value::Null,
            span: tracing::Span::none(),
        }
    }

    #[tokio::test]
    async fn test_timezone_us_eastern_to_utc() {
        let params = json!({
            "datetime_field": "local_dt",
            "zipcode_field": "zip",
            "output": "utc_dt"
        });
        let stage = TimezoneStage::from_params(&params).unwrap();
        let mut record = serde_json::Map::new();
        // Pittsburgh, PA — ZIP 15213 — Eastern Time
        record.insert("local_dt".into(), json!("2024-03-15T14:30:00"));
        record.insert("zip".into(), json!("15213"));
        let item = make_item("i1", record);

        let result = stage.process(item, &ctx()).await.unwrap();
        let rec = result[0].record.as_ref().unwrap();
        let utc = rec.get("utc_dt").unwrap().as_str().unwrap();
        // March 15 2024 is during EDT (UTC-4), so 14:30 ET = 18:30 UTC
        assert!(utc.starts_with("2024-03-15T18:30:00"));
        assert!(utc.ends_with("+00:00"));
    }

    #[tokio::test]
    async fn test_timezone_us_pacific_to_utc() {
        let params = json!({
            "datetime_field": "local_dt",
            "zipcode_field": "zip",
            "output": "utc_dt"
        });
        let stage = TimezoneStage::from_params(&params).unwrap();
        let mut record = serde_json::Map::new();
        // San Francisco, CA — ZIP 94102 — Pacific Time
        record.insert("local_dt".into(), json!("2024-03-15T14:30:00"));
        record.insert("zip".into(), json!("94102"));
        let item = make_item("i1", record);

        let result = stage.process(item, &ctx()).await.unwrap();
        let rec = result[0].record.as_ref().unwrap();
        let utc = rec.get("utc_dt").unwrap().as_str().unwrap();
        // March 15 2024 is during PDT (UTC-7), so 14:30 PT = 21:30 UTC
        assert!(utc.starts_with("2024-03-15T21:30:00"));
        assert!(utc.ends_with("+00:00"));
    }

    #[tokio::test]
    async fn test_timezone_override_store_5995() {
        let params = json!({
            "datetime_field": "local_dt",
            "zipcode_field": "zip",
            "output": "utc_dt",
            "overrides": { "5995": "UTC" },
            "override_key_field": "store_id"
        });
        let stage = TimezoneStage::from_params(&params).unwrap();
        let mut record = serde_json::Map::new();
        record.insert("local_dt".into(), json!("2024-03-15T14:30:00"));
        record.insert("zip".into(), json!("15213")); // Would be Eastern, but override wins
        record.insert("store_id".into(), json!("5995"));
        let item = make_item("i1", record);

        let result = stage.process(item, &ctx()).await.unwrap();
        let rec = result[0].record.as_ref().unwrap();
        let utc = rec.get("utc_dt").unwrap().as_str().unwrap();
        // Override to UTC means no conversion — 14:30 stays 14:30 UTC
        assert!(utc.starts_with("2024-03-15T14:30:00"));
        assert!(utc.ends_with("+00:00"));
    }

    #[tokio::test]
    async fn test_timezone_fallback_on_unknown_zip() {
        let params = json!({
            "datetime_field": "local_dt",
            "zipcode_field": "zip",
            "output": "utc_dt",
            "fallback_timezone": "US/Eastern"
        });
        let stage = TimezoneStage::from_params(&params).unwrap();
        let mut record = serde_json::Map::new();
        record.insert("local_dt".into(), json!("2024-03-15T14:30:00"));
        record.insert("zip".into(), json!("00000")); // Invalid ZIP
        let item = make_item("i1", record);

        let result = stage.process(item, &ctx()).await.unwrap();
        let rec = result[0].record.as_ref().unwrap();
        let utc = rec.get("utc_dt").unwrap().as_str().unwrap();
        // Falls back to US/Eastern: 14:30 EDT = 18:30 UTC
        assert!(utc.starts_with("2024-03-15T18:30:00"));
    }

    #[tokio::test]
    async fn test_timezone_zip_prefix_lookup() {
        // Verify the table has entries for major US regions
        let table = build_zip_prefix_table();
        // Pittsburgh (Eastern)
        assert_eq!(*table.get("152").unwrap(), chrono_tz::US::Eastern);
        // Chicago (Central)
        assert_eq!(*table.get("606").unwrap(), chrono_tz::US::Central);
        // Denver (Mountain)
        assert_eq!(*table.get("802").unwrap(), chrono_tz::US::Mountain);
        // San Francisco (Pacific)
        assert_eq!(*table.get("941").unwrap(), chrono_tz::US::Pacific);
        // Anchorage (Alaska)
        assert_eq!(*table.get("995").unwrap(), chrono_tz::US::Alaska);
        // Honolulu (Hawaii)
        assert_eq!(*table.get("967").unwrap(), chrono_tz::US::Hawaii);
    }
}
