//! O armazenamento do `vamo_fogao` em SQLite.
//!
//! Adaptador da porta de armazenamento que o `vamo-core` define. O banco fica
//! num arquivo local, e as migrações são arquivos `.sql` versionados,
//! aplicados em ordem com `PRAGMA user_version`.
