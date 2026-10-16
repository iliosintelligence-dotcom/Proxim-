//! Shared industry taxonomy and category matching (engine-spec 7.1).
//!
//! A category code is one to four dot-separated segments, each 1..=24 of `[a-z0-9_]`, at most 64
//! bytes in all: `health`, `health.hospital`, `food.cafe`. A pattern matches a category when it
//! equals it or is a whole-segment prefix of it: `health` matches `health.hospital`, not
//! `healthy.food`. Matching never looks at the user: it compares the partner app's static policy
//! with the signpost's category, so no interest profile is built or needed.
//!
//! The engine does not reject a signpost whose category is not in [`TAXONOMY`] (it is matched
//! like any other string); the sender backend is where codes are checked against the list.

/// Longest category code in bytes.
pub const MAX_CATEGORY_BYTES: usize = 64;
/// Most segments in a category code.
pub const MAX_CATEGORY_SEGMENTS: usize = 4;
/// Longest segment in bytes.
pub const MAX_SEGMENT_BYTES: usize = 24;

/// `true` if `code` is a well-formed category code (see the module header).
#[must_use]
pub fn is_valid_code(code: &str) -> bool {
    if code.is_empty() || code.len() > MAX_CATEGORY_BYTES {
        return false;
    }
    let mut segments = 0_usize;
    for seg in code.split('.') {
        segments += 1;
        if seg.is_empty()
            || seg.len() > MAX_SEGMENT_BYTES
            || !seg
                .bytes()
                .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_')
        {
            return false;
        }
    }
    segments <= MAX_CATEGORY_SEGMENTS
}

/// `true` if `pattern` equals `category` or is a whole-segment prefix of it.
#[must_use]
pub fn matches(pattern: &str, category: &str) -> bool {
    category == pattern
        || (category.len() > pattern.len()
            && category.starts_with(pattern)
            && category.as_bytes().get(pattern.len()) == Some(&b'.'))
}

/// The category itself, then each shorter whole-segment prefix: `a.b.c`, `a.b`, `a`. Every
/// pattern that matches `category` is one of these, so a set lookup per prefix finds them all.
pub fn prefixes(category: &str) -> impl Iterator<Item = &str> {
    let cuts = category
        .char_indices()
        .filter(|&(_, c)| c == '.')
        .map(|(i, _)| i)
        .rev();
    core::iter::once(category).chain(cuts.filter_map(move |i| category.get(..i)))
}

/// The shared industry taxonomy: `(code, label)`. Top-level entries are industries; senders pick
/// the most specific code that fits. `alcohol` and `gambling` exist so that partners can block
/// them; nothing in the engine treats them specially.
pub const TAXONOMY: &[(&str, &str)] = &[
    ("alcohol", "Alcohol"),
    ("alcohol.bar", "Bars and pubs"),
    ("alcohol.liquor_store", "Liquor stores"),
    ("auto", "Automotive"),
    ("auto.car_wash", "Car washes"),
    ("auto.dealer", "Car dealers"),
    ("auto.ev_charging", "EV charging"),
    ("auto.fuel", "Fuel stations"),
    ("auto.repair", "Car repair and tyres"),
    ("beauty", "Beauty"),
    ("beauty.salon", "Hair and beauty salons"),
    ("beauty.spa", "Spas"),
    ("civic", "Civic and community"),
    ("civic.community_event", "Community events"),
    ("civic.government", "Government services"),
    ("education", "Education"),
    ("education.course", "Courses and training"),
    ("education.library", "Libraries"),
    ("education.school", "Schools"),
    ("entertainment", "Entertainment"),
    ("entertainment.cinema", "Cinemas"),
    ("entertainment.live_event", "Concerts and live events"),
    ("entertainment.museum", "Museums and galleries"),
    ("finance", "Finance"),
    ("finance.atm", "ATMs"),
    ("finance.bank", "Bank branches"),
    ("finance.insurance", "Insurance"),
    ("fitness", "Fitness"),
    ("fitness.event", "Races and fitness events"),
    ("fitness.gym", "Gyms"),
    ("fitness.studio", "Yoga and fitness studios"),
    ("food", "Food and drink"),
    ("food.bakery", "Bakeries"),
    ("food.cafe", "Cafes"),
    ("food.fast_food", "Fast food and drive-through"),
    ("food.healthy", "Healthy food and juice bars"),
    ("food.restaurant", "Restaurants"),
    ("gambling", "Gambling"),
    ("gambling.betting", "Betting shops"),
    ("gambling.casino", "Casinos"),
    ("health", "Health"),
    ("health.clinic", "Clinics"),
    ("health.dental", "Dentists"),
    ("health.hospital", "Hospitals"),
    ("health.outreach", "Health outreach and screening events"),
    ("health.pharmacy", "Pharmacies"),
    ("health.wellness", "Wellness"),
    ("home", "Home"),
    ("home.hardware", "Hardware and DIY"),
    ("home.services", "Home services"),
    ("pets", "Pets"),
    ("pets.store", "Pet stores"),
    ("pets.vet", "Vets"),
    ("real_estate", "Real estate"),
    ("real_estate.open_house", "Open houses"),
    ("retail", "Retail"),
    ("retail.electronics", "Electronics"),
    ("retail.fashion", "Fashion"),
    ("retail.grocery", "Groceries and supermarkets"),
    ("retail.sportswear", "Sportswear"),
    ("travel", "Travel"),
    ("travel.airport", "Airports"),
    ("travel.attraction", "Attractions"),
    ("travel.hotel", "Hotels"),
    ("travel.transit", "Public transport"),
];

/// `true` if `code` is listed in [`TAXONOMY`].
#[must_use]
pub fn is_known(code: &str) -> bool {
    TAXONOMY.binary_search_by(|(c, _)| (*c).cmp(code)).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    #[test]
    fn whole_segment_prefixes_match_and_nothing_else_does() {
        assert!(matches("health", "health"));
        assert!(matches("health", "health.hospital"));
        assert!(matches("health.hospital", "health.hospital"));
        assert!(!matches("health.hospital", "health"));
        assert!(!matches("health", "healthy.food"));
        assert!(!matches("health", "healthy"));
        assert!(!matches("health.h", "health.hospital"));
        assert!(!matches("", "health"));
        assert!(matches("a.b", "a.b.c.d"));
    }

    #[test]
    fn prefixes_are_longest_first_and_cover_every_match() {
        let p: Vec<&str> = prefixes("a.b.c").collect();
        assert_eq!(p, ["a.b.c", "a.b", "a"]);
        let p: Vec<&str> = prefixes("cafe").collect();
        assert_eq!(p, ["cafe"]);
        // odd but legal input: every prefix at a dot, including empty segments
        let p: Vec<&str> = prefixes(".x.").collect();
        assert_eq!(p, [".x.", ".x", ""]);
        for (code, _) in TAXONOMY {
            for pre in prefixes(code) {
                assert!(matches(pre, code), "{pre} vs {code}");
            }
        }
    }

    #[test]
    fn code_syntax() {
        for ok in ["a", "food.cafe", "a.b.c.d", "x_1.y2", &"a".repeat(24)] {
            assert!(is_valid_code(ok), "{ok}");
        }
        for bad in [
            "",
            ".",
            "a.",
            ".a",
            "a..b",
            "A",
            "food.Cafe",
            "a.b.c.d.e",
            "caf\u{e9}",
            "a-b",
            "a b",
            &"a".repeat(25),
        ] {
            assert!(!is_valid_code(bad), "{bad}");
        }
        let long = ["abcdefghijklmnop"; 4].join("."); // 67 bytes
        assert!(!is_valid_code(&long));
    }

    #[test]
    fn taxonomy_is_sorted_unique_valid_and_every_parent_is_listed() {
        assert!(TAXONOMY.windows(2).all(|w| w[0].0 < w[1].0));
        for (code, label) in TAXONOMY {
            assert!(is_valid_code(code), "{code}");
            assert!(!label.is_empty());
            if let Some((parent, _)) = code.rsplit_once('.') {
                assert!(is_known(parent), "{code} has no parent");
            }
        }
        assert!(is_known("health.hospital") && !is_known("health.hosp"));
    }
}
