//! Identifiants opaques, stables et triables : ULID maison.
//!
//! Aucune dépendance à un générateur aléatoire système : l'entropie provient
//! d'un xorshift amorcé par l'horloge et un compteur, ce qui évite toute
//! dépendance `getrandom` et fonctionne à l'identique en natif et en WASM.
//! Les identifiants ne sont pas des secrets ; seule l'unicité compte.

use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::str::FromStr;

const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";
const MASK80: u128 = (1u128 << 80) - 1;

/// Identifiant d'objet du référentiel (26 caractères Crockford base32).
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObjectId([u8; 26]);

impl ObjectId {
    pub fn as_str(&self) -> &str {
        std::str::from_utf8(&self.0).unwrap_or("")
    }

    /// Horodatage encodé dans les 48 premiers bits, en millisecondes UTC.
    pub fn timestamp_ms(&self) -> u64 {
        let v = decode(self.as_str()).unwrap_or(0);
        (v >> 80) as u64
    }

    pub const fn from_parts(timestamp_ms: u64, entropy: u128) -> Self {
        let v = (((timestamp_ms & 0xFFFF_FFFF_FFFF) as u128) << 80) | (entropy & MASK80);
        let mut out = [0u8; 26];
        let mut rest = v;
        let mut i = 26;
        while i > 0 {
            i -= 1;
            out[i] = ALPHABET[(rest & 31) as usize];
            rest >>= 5;
        }
        ObjectId(out)
    }
}

impl fmt::Display for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl fmt::Debug for ObjectId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ObjectId({})", self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("identifiant invalide : {0}")]
pub struct IdError(pub String);

impl FromStr for ObjectId {
    type Err = IdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 26 {
            return Err(IdError(format!("longueur {} au lieu de 26", s.len())));
        }
        let upper = s.to_ascii_uppercase();
        decode(&upper).ok_or_else(|| IdError("caractère hors alphabet Crockford".into()))?;
        let mut out = [0u8; 26];
        out.copy_from_slice(upper.as_bytes());
        Ok(ObjectId(out))
    }
}

impl Serialize for ObjectId {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for ObjectId {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        raw.parse().map_err(de::Error::custom)
    }
}

fn decode(s: &str) -> Option<u128> {
    let mut acc: u128 = 0;
    for (i, ch) in s.bytes().enumerate() {
        let digit = match ch {
            b'0'..=b'9' => ch - b'0',
            b'A'..=b'H' => ch - b'A' + 10,
            b'J' | b'K' => ch - b'J' + 18,
            b'M' | b'N' => ch - b'M' + 20,
            b'P'..=b'T' => ch - b'P' + 22,
            b'V'..=b'Z' => ch - b'V' + 27,
            b'a'..=b'z' => return decode(&s.to_ascii_uppercase()),
            _ => return None,
        };
        if i == 0 && digit > 7 {
            return None;
        }
        acc = acc.checked_shl(5)? | digit as u128;
    }
    Some(acc)
}

/// Générateur monotone, sans I/O, utilisable en WASM.
#[derive(Debug, Clone)]
pub struct IdGenerator {
    state: u64,
    last_ms: u64,
    counter: u128,
    base: u128,
}

impl IdGenerator {
    pub fn new(seed: u64) -> Self {
        Self {
            state: seed | 1,
            last_ms: 0,
            counter: 0,
            base: 0,
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.state = x;
        x
    }

    fn fresh_base(&mut self) -> u128 {
        let hi = self.next_u64() as u128;
        let lo = self.next_u64() as u128;
        ((hi << 64) | lo) & MASK80
    }

    pub fn next_id(&mut self, now_ms: u64) -> ObjectId {
        if now_ms != self.last_ms {
            self.last_ms = now_ms;
            self.counter = 0;
            self.base = self.fresh_base();
        } else {
            // Monotonie garantie à l'intérieur d'une même milliseconde.
            self.counter += 1;
        }
        let entropy = (self.base + self.counter) & MASK80;
        ObjectId::from_parts(now_ms, entropy)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_order() {
        let mut gen = IdGenerator::new(42);
        let a = gen.next_id(1_700_000_000_000);
        let b = gen.next_id(1_700_000_000_001);
        assert_eq!(a.as_str().len(), 26);
        assert!(a < b, "un ULID plus récent doit trier après");
        assert_eq!(a.timestamp_ms(), 1_700_000_000_000);
        assert_eq!(ObjectId::from_str(a.as_str()).unwrap(), a);
    }

    #[test]
    fn monotonic_inside_same_millisecond() {
        let mut gen = IdGenerator::new(7);
        let ids: Vec<_> = (0..100).map(|_| gen.next_id(1_700_000_000_000)).collect();
        let mut sorted = ids.clone();
        sorted.sort();
        assert_eq!(ids, sorted);
        assert_eq!(
            ids.iter().collect::<std::collections::BTreeSet<_>>().len(),
            100
        );
    }

    #[test]
    fn rejects_bad_ids() {
        assert!(ObjectId::from_str("trop court").is_err());
        assert!(ObjectId::from_str("01ARZ3NDEKTSV4RRFFQ69G5FAV").is_ok());
        assert!(ObjectId::from_str("UUUUUUUUUUUUUUUUUUUUUUUUUU").is_err());
    }

    #[test]
    fn serde_is_a_plain_string() {
        let id = ObjectId::from_parts(1, 5);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, format!("\"{id}\""));
        assert_eq!(serde_json::from_str::<ObjectId>(&json).unwrap(), id);
    }
}
