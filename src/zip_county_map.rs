use std::collections::HashMap;

#[derive(Debug)]
pub struct ZipCountyInfo {
    pub county_code: String,
    pub division: String,
    pub fips_code: String,  // This will now store just the 3-digit county FIPS code
    pub county_name: String, // Added county name field
}

pub fn load_zip_county_map() -> HashMap<String, ZipCountyInfo> {
    let mut map = HashMap::new();
    
    // Create a mapping of county_code to county name and FIPS code
    let mut county_to_info = HashMap::new();
    
    // Populate county to FIPS and name mapping
    let county_codes: Vec<(String, String, String)> = vec![
        ("01".to_string(), "001".to_string(), "Adams County".to_string()),
        ("02".to_string(), "003".to_string(), "Asotin County".to_string()),
        ("03".to_string(), "005".to_string(), "Benton County".to_string()),
        ("04".to_string(), "007".to_string(), "Chelan County".to_string()),
        ("05".to_string(), "009".to_string(), "Clallam County".to_string()),
        ("06".to_string(), "011".to_string(), "Clark County".to_string()),
        ("07".to_string(), "013".to_string(), "Columbia County".to_string()),
        ("08".to_string(), "015".to_string(), "Cowlitz County".to_string()),
        ("09".to_string(), "017".to_string(), "Douglas County".to_string()),
        ("10".to_string(), "019".to_string(), "Ferry County".to_string()),
        ("11".to_string(), "021".to_string(), "Franklin County".to_string()),
        ("12".to_string(), "023".to_string(), "Garfield County".to_string()),
        ("13".to_string(), "025".to_string(), "Grant County".to_string()),
        ("14".to_string(), "027".to_string(), "Grays Harbor County".to_string()),
        ("15".to_string(), "029".to_string(), "Island County".to_string()),
        ("16".to_string(), "031".to_string(), "Jefferson County".to_string()),
        ("17".to_string(), "033".to_string(), "King County".to_string()),
        ("18".to_string(), "035".to_string(), "Kitsap County".to_string()),
        ("19".to_string(), "037".to_string(), "Kittitas County".to_string()),
        ("20".to_string(), "039".to_string(), "Klickitat County".to_string()),
        ("21".to_string(), "041".to_string(), "Lewis County".to_string()),
        ("22".to_string(), "043".to_string(), "Lincoln County".to_string()),
        ("23".to_string(), "045".to_string(), "Mason County".to_string()),
        ("24".to_string(), "047".to_string(), "Okanogan County".to_string()),
        ("25".to_string(), "049".to_string(), "Pacific County".to_string()),
        ("26".to_string(), "051".to_string(), "Pend Oreille County".to_string()),
        ("27".to_string(), "053".to_string(), "Pierce County".to_string()),
        ("28".to_string(), "055".to_string(), "San Juan County".to_string()),
        ("29".to_string(), "057".to_string(), "Skagit County".to_string()),
        ("30".to_string(), "059".to_string(), "Skamania County".to_string()),
        ("31".to_string(), "061".to_string(), "Snohomish County".to_string()),
        ("32".to_string(), "063".to_string(), "Spokane County".to_string()),
        ("33".to_string(), "065".to_string(), "Stevens County".to_string()),
        ("34".to_string(), "067".to_string(), "Thurston County".to_string()),
        ("35".to_string(), "069".to_string(), "Wahkiakum County".to_string()),
        ("36".to_string(), "071".to_string(), "Walla Walla County".to_string()),
        ("37".to_string(), "073".to_string(), "Whatcom County".to_string()),
        ("38".to_string(), "075".to_string(), "Whitman County".to_string()),
        ("39".to_string(), "077".to_string(), "Yakima County".to_string()),
    ];
    
    // Populate the county_to_info map
    for (county_code, county_fips, county_name) in county_codes {
        county_to_info.insert(county_code, (county_fips, county_name));
    }
    
    // Load the hardcoded zip:county_code:division data
    let zip_data = vec![
        "98602:20:A", "98605:20:A", "98068:19:A", "98613:20:A", "98617:20:A",
        "98619:20:A", "98620:20:A", "98622:20:A", "98623:20:A", "98628:20:A",
        "98635:20:A", "98650:20:A", "98656:20:A", "98670:20:A", "98672:20:A",
        "98673:20:A", "98801:04:B", "98802:09:B", "98807:04:B", "98811:04:B",
        "98812:24:B", "98813:09:B", "98814:24:B", "98815:04:B", "98816:04:B",
        "98817:04:B", "98819:24:B", "98821:04:B", "98822:04:B", "98823:13:B",
        "98824:13:B", "98826:04:B", "98827:24:B", "98828:04:B", "98829:24:B",
        "98830:09:B", "98831:04:B", "98832:13:B", "98833:24:B", "98834:24:B",
        "98836:04:B", "98837:13:B", "98840:24:B", "98841:24:B", "98843:09:B",
        "98844:24:B", "98845:09:B", "98846:24:B", "98847:04:B", "98848:13:B",
        "98849:24:B", "98850:09:B", "98851:13:B", "98852:04:B", "98853:13:B",
        "98855:24:B", "98856:24:B", "98857:13:B", "98858:09:B", "98859:24:B",
        "98860:13:B", "98862:24:B", "98901:39:A", "98902:39:A", "98903:39:A",
        "98904:39:A", "98907:39:A", "98908:39:A", "98909:39:A", "98920:39:A",
        "98921:39:A", "98922:19:A", "98923:39:A", "98925:19:A", "98926:19:A",
        "98930:39:A", "98932:39:A", "98933:39:A", "98934:19:A", "98935:39:A",
        "98936:39:A", "98937:39:A", "98938:39:A", "98939:39:A", "98940:19:A",
        "98941:19:A", "98942:39:A", "98943:19:A", "98944:39:A", "98946:19:A",
        "98947:39:A", "98948:39:A", "98950:19:A", "98951:39:A", "98952:39:A",
        "98953:39:A", "99001:32:B", "99003:32:B", "99004:32:B", "99005:32:B",
        "99006:32:B", "99008:22:B", "99009:32:B", "99011:32:B", "99012:32:B",
        "99013:33:B", "99014:32:B", "99016:32:B", "99017:38:B", "99018:32:B",
        "99019:32:B", "99020:32:B", "99021:32:B", "99022:32:B", "99023:32:B",
        "99025:32:B", "99026:32:B", "99027:32:B", "99029:22:B", "99030:32:B",
        "99031:32:B", "99032:22:B", "99033:38:B", "99034:33:B", "99036:32:B",
        "99037:32:B", "99039:32:B", "99040:33:B", "99101:33:B", "99102:38:B",
        "99103:22:B", "99104:38:B", "99105:01:B", "99107:10:B", "99109:33:B",
        "99110:33:B", "99111:38:B", "99113:38:B", "99114:33:B", "99115:13:B",
        "99116:24:B", "99117:22:B", "99118:10:B", "99119:26:B", "99121:10:B",
        "99122:22:B", "99123:13:B", "99124:24:B", "99125:38:B", "99126:33:B",
        "99128:38:B", "99129:33:B", "99130:38:B", "99131:33:B", "99133:13:B",
        "99134:22:B", "99135:13:B", "99136:38:B", "99137:33:B", "99138:10:B",
        "99139:26:B", "99140:10:B", "99141:33:B", "99143:38:B", "99144:22:B",
        "99146:10:B", "99147:22:B", "99148:33:B", "99149:38:B", "99150:10:B",
        "99151:33:B", "99152:26:B", "99153:26:B", "99154:22:B", "99155:24:B",
        "99156:26:B", "99157:33:B", "99158:38:B", "99159:22:B", "99160:10:B",
        "99161:38:B", "99163:38:B", "99164:38:B", "99166:10:B", "99167:33:B",
        "99169:01:B", "99170:32:B", "99171:38:B", "99173:33:B", "99174:38:B",
        "99176:38:B", "99179:38:B", "99180:26:B", "99181:33:B", "99185:22:B",
        "99201:32:B", "99202:32:B", "99203:32:B", "99204:32:B", "99205:32:B",
        "99206:32:B", "99207:32:B", "99208:32:B", "99209:32:B", "99210:32:B",
        "99211:32:B", "99212:32:B", "99213:32:B", "99214:32:B", "99215:32:B",
        "99216:32:B", "99217:32:B", "99218:32:B", "99219:32:B", "99220:32:B",
        "99223:32:B", "99224:32:B", "99228:32:B", "99251:32:B", "99252:32:B",
        "99254:32:B", "99256:32:B", "99258:32:B", "99260:32:B", "99301:11:A",
        "99302:11:A", "99320:03:A", "99321:13:B", "99322:20:A", "99323:36:A",
        "99324:36:A", "99326:11:A", "99328:07:B", "99329:36:A", "99330:11:A",
        "99333:38:B", "99335:11:A", "99336:03:A", "99337:03:A", "99338:03:A",
        "99341:01:B", "99343:11:A", "99344:01:A", "99345:03:A", "99346:03:A",
        "99347:12:B", "99348:36:A", "99349:13:B", "99350:03:A", "99352:03:A",
        "99353:03:A", "99354:03:A", "99356:20:A", "99357:13:B", "99359:07:B",
        "99360:36:A", "99361:36:A", "99362:36:A", "99363:36:A", "99371:01:B",
        "99401:02:B", "99402:02:B", "99403:02:B",
    ];
    
    // Parse each zip code entry and add to the map with FIPS code and county name
    for entry in zip_data {
        let parts: Vec<&str> = entry.split(':').collect();
        if parts.len() == 3 {
            let zip = parts[0].to_string();
            let county_code = parts[1].to_string();
            let division = parts[2].to_string();
            
            // Look up the FIPS code and county name for this county
            let (fips_code, county_name) = match county_to_info.get(&county_code) {
                Some((fips, name)) => (fips.clone(), name.clone()),
                None => (String::new(), String::new()) // Empty strings if not found
            };
            
            map.insert(zip, ZipCountyInfo { 
                county_code,
                division,
                fips_code,
                county_name
            });
        }
    }
    
    map
}

// Get the county code for a zip plus 4 by matching first 5 digits
pub fn get_county_code_for_zip(zip_plus_4: &str, zip_county_map: &HashMap<String, ZipCountyInfo>) -> Option<String> {
    // Extract first 5 digits of zip_plus_4
    if zip_plus_4.len() >= 5 {
        let zip5 = &zip_plus_4[0..5];
        if let Some(info) = zip_county_map.get(zip5) {
            return Some(info.county_code.clone());
        }
    }
    None
}

// Get the FIPS code for a zip plus 4 by matching first 5 digits
pub fn get_fips_code_for_zip(zip_plus_4: &str, zip_county_map: &HashMap<String, ZipCountyInfo>) -> Option<String> {
    // Extract first 5 digits of zip_plus_4
    if zip_plus_4.len() >= 5 {
        let zip5 = &zip_plus_4[0..5];
        if let Some(info) = zip_county_map.get(zip5) {
            return Some(info.fips_code.clone());
        }
    }
    None
}

// Get the county name for a zip plus 4 by matching first 5 digits
pub fn get_county_name_for_zip(zip_plus_4: &str, zip_county_map: &HashMap<String, ZipCountyInfo>) -> Option<String> {
    // Extract first 5 digits of zip_plus_4
    if zip_plus_4.len() >= 5 {
        let zip5 = &zip_plus_4[0..5];
        if let Some(info) = zip_county_map.get(zip5) {
            return Some(info.county_name.clone());
        }
    }
    None
}