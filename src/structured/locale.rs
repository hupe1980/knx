//! Language and region identifiers listed in Tables 3 and 4 of KNX DPT v02.02.01.
use crate::{CodecError, codecs::bytes};
const LANGUAGES: &[u8] = concat!(
    "aaabaeafakamanarasavayazbabebgbhbibmbnbobrbscacechcocrcscucvcyda",
    "dedvdzeeeleneoeseteufafffifjfofrfygagdglgngugvhahehihohrhthuhyhz",
    "iaidieigiiikioisitiujajvkakgkikjkkklkmknkokrkskukvkwkylalblgliln",
    "loltlulvmgmhmimkmlmnmomrmsmtmynanbndnengnlnnnonrnvnyocojomorospa",
    "piplpsptqurmrnrorurwsascsdsesgshsiskslsmsnsosqsrssstsusvswtatetg",
    "thtitktltntotrtstttwtyugukuruzvevivowawoxhyiyozazhzu",
)
.as_bytes();
const REGIONS: &[u8] = concat!(
    "ADAEAFAGAIALAMANAOAQARASATAUAWAXAZBABBBDBEBFBGBHBIBJBLBMBNBOBRBS",
    "BTBVBWBYBZCACCCDCFCGCHCICKCLCMCNCOCRCSCUCVCXCYCZDEDJDKDMDODZECEE",
    "EGEHERESETFIFJFKFMFOFRGAGBGDGEGFGGGHGIGLGMGNGPGQGRGSGTGUGWGYHKHM",
    "HNHRHTHUIDIEILIMINIOIQIRISITJEJMJOJPKEKGKHKIKMKNKPKRKWKYKZLALBLC",
    "LILKLRLSLTLULVLYMAMCMDMEMFMGMHMKMLMMMNMOMPMQMRMSMTMUMVMWMXMYMZNA",
    "NCNENFNGNINLNONPNRNUNZOMPAPEPFPGPHPKPLPMPNPRPSPTPWPYQARERORSRURW",
    "SASBSCSDSESGSHSISJSKSLSMSNSOSRSTSVSYSZTCTDTFTGTHTJTKTLTMTNTOTRTT",
    "TVTWTZUAUGUMUSUYUZVAVCVEVGVIVNVUWFWSYEYTZAZMZWZZ",
)
.as_bytes();

/// Edition-bound catalogue. Regions include ZZ (unspecified region).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Alpha2 {
    Language,
    Region,
}
impl Alpha2 {
    /// Normalize received or user-supplied codes. Languages transmit lowercase and regions uppercase.
    pub fn normalize(self, input: &[u8]) -> Result<[u8; 2], CodecError> {
        let code = bytes::<2>(input)?;
        let (canonical, table) = match self {
            Self::Language => (code.map(|c| c.to_ascii_lowercase()), LANGUAGES),
            Self::Region => (code.map(|c| c.to_ascii_uppercase()), REGIONS),
        };
        if table.as_chunks::<2>().0.contains(&canonical) {
            Ok(canonical)
        } else {
            Err(CodecError::OutsideRange)
        }
    }
}
/// DPT 231.001 normalized four-byte locale. Use ZZ for unspecified region.
pub fn normalize_locale(input: &[u8]) -> Result<[u8; 4], CodecError> {
    let [a, b, c, d] = bytes(input)?;
    let [a, b] = Alpha2::Language.normalize(&[a, b])?;
    let [c, d] = Alpha2::Region.normalize(&[c, d])?;
    Ok([a, b, c, d])
}
