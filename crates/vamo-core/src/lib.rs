//! O domínio do `vamo_fogao`: tarefas, status, encaixe, regras do tempo,
//! brechas e o título no padrão do caderno.
//!
//! É o centro da arquitetura hexagonal. Este crate não faz IO e não lê o
//! relógio: tudo que vem de fora (banco de dados, agenda, terminal, a hora
//! atual) entra por parâmetro ou por uma trait que um adaptador implementa.
//! Por isso ele é o crate mais testado do projeto.
//!
//! Regra de dependência: o `vamo-core` não depende de nenhum outro crate do
//! workspace. São os outros que dependem dele.
