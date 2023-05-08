use crate::Error;

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct MigrationHash([u8; 32]);

impl MigrationHash {
    pub fn from_name_and_text(name: &str, text: &str) -> Self {
        use sha3::{Digest, Sha3_256};

        let mut hasher = Sha3_256::new();

        hasher.update(b"NAME");
        hasher.update(name.as_bytes());

        hasher.update(b"TEXT");
        hasher.update(text.as_bytes());

        MigrationHash(hasher.finalize().into())
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0[..]
    }
}

impl std::fmt::Display for MigrationHash {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for byte in &self.0 {
            write!(f, "{:02x}", byte)?;
        }

        Ok(())
    }
}

impl std::fmt::Debug for MigrationHash {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for byte in self.0.iter().take(4) {
            write!(f, "{:02x}", byte)?;
        }

        Ok(())
    }
}

impl std::str::FromStr for MigrationHash {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        hash_from_str(s).map(MigrationHash)
    }
}

fn hash_from_str(s: &str) -> Result<[u8; 32], crate::Error> {
    if !s.is_ascii() {
        return Err(Error::invalid_hash("not ascii"));
    }

    let mut bytes = [0u8; 32];

    for (i, hex) in s.trim().as_bytes().chunks(2).enumerate() {
        let hex = std::str::from_utf8(hex).unwrap();
        match u8::from_str_radix(hex, 16) {
            Err(_) => return Err(Error::invalid_hash("not hex")),
            Ok(_) if i > 31 => return Err(Error::invalid_hash("too long")),
            Ok(byte) => bytes[i] = byte,
        }
    }

    Ok(bytes)
}

#[derive(Default, Clone, PartialEq, Eq, Hash)]
pub struct CommitHash([u8; 32]);

impl CommitHash {
    pub fn from_parent_and_hash(parent: &CommitHash, hash: &MigrationHash) -> CommitHash {
        use sha3::{Digest, Sha3_256};

        let mut hasher = Sha3_256::new();

        hasher.update(b"PARENT");
        hasher.update(parent.as_bytes());

        hasher.update(b"HASH");
        hasher.update(hash.as_bytes());

        CommitHash(hasher.finalize().into())
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.0[..]
    }

    pub fn is_zero(&self) -> bool {
        self.as_bytes().iter().all(|b| *b == 0)
    }
}

impl std::fmt::Display for CommitHash {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for byte in &self.0 {
            write!(f, "{:02x}", byte)?;
        }

        Ok(())
    }
}

impl std::fmt::Debug for CommitHash {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for byte in self.0.iter().take(4) {
            write!(f, "{:02x}", byte)?;
        }

        Ok(())
    }
}

impl std::str::FromStr for CommitHash {
    type Err = crate::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        hash_from_str(s).map(CommitHash)
    }
}
