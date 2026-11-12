# 0010. Ler o Google Agenda pela API, com OAuth e atrás de uma feature

- **Status:** aceita
- **Data:** 2026-11-12

## Contexto

O link ICS do Google (ADR 0008) às vezes leva horas pra mostrar um evento novo. A API do Google Calendar responde na hora, mas pede login (OAuth) e um cliente OAuth criado por quem usa no Google Cloud. A tarefa pede que o app funcione igual com uma porta ou com a outra, e que os testes não usem a rede.

## Decisão

- A segunda porta mora no `vamo-agenda`, atrás da feature `google`, desligada por padrão. O binário repassa a feature (`cargo install --path crates/vamo_fogao --features google`). Sem ela, `fonte = "google"` e o `vamo conectar-google` explicam como ligar.
- O login é o OAuth de aplicativo de desktop: o `vamo conectar-google` abre uma porta em `127.0.0.1`, mostra o endereço de autorização (e tenta abrir o navegador), recebe o código e troca pelo token. O escopo é só leitura (`calendar.readonly`), e o `state` aleatório confere que o retorno é do mesmo pedido.
- O token que dura (`refresh_token`) fica em `$XDG_CONFIG_HOME/vamo_fogao/google-token.json`, com permissão `0600`: fora do repositório e fora do banco de tarefas. Apagar o arquivo desconecta. Nenhuma mensagem de erro mostra token, segredo ou URL.
- A API já expande a recorrência (`singleEvents=true`). O `GoogleCalendar` transforma cada ocorrência no mesmo `Appointment` do leitor do ICS, com o UID da série nas ocorrências de um recorrente (é ele que a marcação das fixas usa, ADR 0009), e implementa a mesma porta `Agenda`. No binário, as duas viram um `Source` só.
- A resposta da API vai pra uma cópia local (`agenda-google.json`), como o ICS: a TUI lê a cópia e abre sem rede.
- O JSON é lido com o `serde_json`, campo por campo e sem o derive do serde, no mesmo estilo do leitor de TOML. É a única dependência nova, MIT ou Apache-2.0.

## Alternativas consideradas

- **Os crates `google-calendar3` e `yup-oauth2`:** trazem o tokio e o hyper pra fazer duas requisições por sync, num binário que hoje não tem runtime assíncrono.
- **PKCE no login:** pede SHA-256 e base64 (duas dependências a mais). No cliente de desktop o Google aceita o segredo do cliente, e a porta local só aceita conexão da própria máquina.
- **Guardar o token no banco:** misturaria credencial com tarefa, e o banco é o que vai ser exportado na sincronia.

## Consequências

- O fluxo de login só é testado nas partes que não precisam de rede (o endereço, o retorno, o token gravado com a permissão certa); a primeira conexão de verdade é com quem usa, com um cliente OAuth próprio.
- Os testes leem uma resposta gravada da API, com dados inventados, e comparam com a mesma semana escrita em ICS: as duas portas têm que dar os mesmos compromissos.
- Quem usa precisa criar o cliente OAuth no Google Cloud e pôr `google_cliente` e `google_segredo` na config. O livro explica o passo a passo.
