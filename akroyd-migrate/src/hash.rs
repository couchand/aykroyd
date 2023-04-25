use crate::Error;

use sha3::{Digest, Sha3_256};

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct MigrationHash([u8; 32]);

impl AsRef<[u8]> for MigrationHash {
    fn as_ref(&self) -> &[u8] {
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
    type Err = Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if !s.is_ascii() {
            return Err(Error::invalid_hash("not ascii"));
        }

        println!("parsing:");
        println!("  - {s}");

        let mut bytes = [0u8; 32];

        for (i, hex) in s.trim().as_bytes().chunks(2).enumerate() {
            let hex = std::str::from_utf8(hex).unwrap();
            match u8::from_str_radix(hex, 16) {
                Err(_) => return Err(Error::invalid_hash("not hex")),
                Ok(_) if i > 31 => return Err(Error::invalid_hash("too long")),
                Ok(byte) => bytes[i] = byte,
            }
        }

        Ok(MigrationHash(bytes))
    }
}

impl postgres_types::ToSql for MigrationHash {
    fn to_sql(
        &self,
        ty: &postgres_types::Type,
        buf: &mut bytes::BytesMut,
    ) -> Result<postgres_types::IsNull, Box<(dyn std::error::Error + Send + Sync + 'static)>> {
        self.to_string().to_sql(ty, buf)
    }
    fn accepts(ty: &postgres_types::Type) -> bool {
        <String as postgres_types::ToSql>::accepts(ty)
    }

    postgres_types::to_sql_checked!();
}

impl<'a> postgres_types::FromSql<'a> for MigrationHash {
    fn from_sql(
        ty: &postgres_types::Type,
        buf: &'a [u8],
    ) -> Result<Self, Box<(dyn std::error::Error + Send + Sync + 'static)>> {
        let string = <String as postgres_types::FromSql>::from_sql(ty, buf)?;
        string.parse().map_err(Into::into)
    }
    fn accepts(ty: &postgres_types::Type) -> bool {
        <String as postgres_types::FromSql>::accepts(ty)
    }
}

impl MigrationHash {
    pub const ZERO: MigrationHash = MigrationHash([0; 32]);

    pub fn is_zero(&self) -> bool {
        self == &MigrationHash::ZERO
    }

    pub fn from_content<S: AsRef<str>>(string: S) -> MigrationHash {
        let mut hasher = Sha3_256::new();

        hasher.update(string.as_ref().as_bytes());

        MigrationHash(hasher.finalize().into())
    }

    pub fn from_deps_and_hash(deps: &[MigrationHash], hash: &MigrationHash) -> MigrationHash {
        MigrationHash::from_deps_and_hash_opt(deps, Some(hash))
    }

    pub fn from_deps_and_hash_opt(
        deps: &[MigrationHash],
        hash: Option<&MigrationHash>,
    ) -> MigrationHash {
        println!("new hash");

        let mut hasher = Sha3_256::new();

        println!("  deps:");
        hasher.update(b"DEPS");

        for dep in deps {
            println!("  - {dep}");
            hasher.update(dep);
        }

        println!("  hash:");
        hasher.update(b"HASH");

        match hash {
            Some(hash) => {
                println!("  - {hash}");
                hasher.update(hash);
            }
            None => {
                println!("  - None");
                hasher.update(b"NONE");
            }
        }

        println!("  result:");
        let result = MigrationHash(hasher.finalize().into());

        println!("  - {result}");
        result
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use hex_literal::hex;

    #[test]
    fn from_content() {
        fn test(content: &str, expected: [u8; 32]) {
            assert_eq!(MigrationHash::from_content(content).0, expected);
        }

        test(
            "abc",
            hex!("3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532"),
        );
    }

    #[test]
    fn from_deps_and_hash() {
        struct Test {
            deps: Vec<[u8; 32]>,
            hash: [u8; 32],
            expected: [u8; 32],
        }

        impl Test {
            fn run(self) {
                let deps = self.deps.into_iter().map(MigrationHash).collect::<Vec<_>>();
                let hash = MigrationHash(self.hash);
                assert_eq!(
                    MigrationHash::from_deps_and_hash(&deps, &hash).0,
                    self.expected
                );
            }
        }

        Test {
            deps: vec![hex!(
                "0000000000000000000000000000000000000000000000000000000000000000"
            )],
            hash: hex!("3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532"),
            expected: hex!("a09c02511937e87ac8764d1f3365e5bbd4e829eddad747ff4d9f18d099992104"),
        }
        .run();
    }

    #[test]
    fn from_deps_and_hash_opt() {
        struct Test {
            deps: Vec<[u8; 32]>,
            hash: Option<[u8; 32]>,
            expected: [u8; 32],
        }

        impl Test {
            fn run(self) {
                let deps = self.deps.into_iter().map(MigrationHash).collect::<Vec<_>>();
                let hash = self.hash.map(MigrationHash);
                assert_eq!(
                    MigrationHash::from_deps_and_hash_opt(&deps, hash.as_ref()).0,
                    self.expected
                );
            }
        }

        Test {
            deps: vec![hex!(
                "0000000000000000000000000000000000000000000000000000000000000000"
            )],
            hash: Some(hex!(
                "3a985da74fe225b2045c172d6bd390bd855f086e3e9d525b46bfe24511431532"
            )),
            expected: hex!("a09c02511937e87ac8764d1f3365e5bbd4e829eddad747ff4d9f18d099992104"),
        }
        .run();

        Test {
            deps: vec![hex!(
                "0000000000000000000000000000000000000000000000000000000000000000"
            )],
            hash: None,
            expected: hex!("b8bfc439e642af332d834ffa6142582b3c7f87f2cc4adede7141ba53cee41e49"),
        }
        .run();
    }

    #[test]
    fn migration_hash_from_str() {
        fn test(input: &str, expected: [u8; 32]) {
            let migration_hash: MigrationHash = input.parse().expect("parse");
            assert_eq!(migration_hash.0, expected);
        }

        test(
            "b8bfc439e642af332d834ffa6142582b3c7f87f2cc4adede7141ba53cee41e49",
            hex!("b8bfc439e642af332d834ffa6142582b3c7f87f2cc4adede7141ba53cee41e49"),
        );
    }
}
