# XkyHold-wd — plano base

Data: 18/11/2026 · Estado: **plano aprovado (decisões W1–W11, 18/11/2026); Fase 0 concluída a 19/11/2026; próxima: Fase 1 (adenda ao formato)**

O XkyHold-wd é a app de Windows da família XkyHold. Faz cópias de segurança cifradas da pasta do utilizador (`%USERPROFILE%`) inteira, ou de partes dela, incluindo `AppData`, ficheiros ocultos, chaves SSH, configurações de programas e ligações. Existe porque nem a XkyHold web (browser, recusa as pastas de sistema e os ocultos) nem o XkyHold-lx (só Linux) servem um utilizador avançado de Windows. Partilha o motor, o formato e o desenho com o XkyHold-lx.

Ler antes de começar: o checkpoint e o `docs/PLAN.md` do `../xkyhold-lx` (decisões L1–L4, arquitetura, armadilhas), o `docs/FORMAT-v2.md` do `../xkyhold` e o `CLAUDE.md` do xkyhold-lx.

## 1. Âmbito

**Dentro:**

- Guardar `%USERPROFILE%` inteira ou pastas e ficheiros dentro dela: `AppData\Roaming`, `AppData\Local`, `.ssh`, `.gitconfig`, `Documents`, `Desktop`, etc. Ao contrário da web, a pasta do utilizador, Documentos e Ambiente de trabalho podem ser escolhidos.
- Repor para o sítio original ou para outra pasta, com atributos, datas e ligações.
- Destinos: Google Drive do utilizador e uma pasta local (disco externo, partilha de rede `\\servidor\pasta`).
- Cópia manual, lembrete e cópia automática opcional.
- Interface gráfica e linha de comandos.

**Fora (regra fixa, W2 e W3):**

- Nada fora de `%USERPROFILE%`: nem outras unidades (`D:\`), nem `C:\ProgramData`, nem `Program Files`, nem o registo do sistema. Não é um backup de sistema nem uma imagem do disco.
- Nunca corre como administrador e nunca pede elevação (UAC).
- Sem Volume Shadow Copy. Ficheiros bloqueados por outro programa (Outlook `.pst`/`.ost` aberto, perfis de browser abertos, `NTUSER.DAT`) são saltados e contados, como os ilegíveis no lx. A app mostra quais foram e sugere fechar o programa.
- Nunca segue ligações ao copiar (ligações simbólicas, junctions, pontos de montagem).
- macOS. Compatibilidade com o Histórico de Ficheiros do Windows, restic ou BackMa.

**Público:** utilizadores avançados de Windows. Tom simples e direto, mas a app mostra caminhos, padrões de exclusão e opções que a web esconde.

## 2. Decisões do proprietário

| # | Tema | Decisão | Data |
|---|---|---|---|
| W1 | Código | **Motor partilhado.** Os crates `core`, `dest-local`, `dest-drive` (e o que servir de `app`/`secrets`) do xkyhold-lx passam a ser multiplataforma (`cfg(windows)` / `cfg(unix)`); o xkyhold-wd depende deles por git com rev fixa. Um só motor: uma correção serve as duas apps | 18/11/2026 |
| W2 | Âmbito | **Só `%USERPROFILE%`**, espelho da regra do lx | 18/11/2026 |
| W3 | Ficheiros bloqueados | **Nunca VSS.** Nunca pede administrador; bloqueados são saltados e contados | 18/11/2026 |
| W4 | Onde testar | **VM Windows 11 no Omarchy** (contentor `omarchy-windows`, imagem `dockurr/windows`: 4 CPUs, 8 GB, disco 64 GB; ecrã em `http://127.0.0.1:8006`, RDP em `127.0.0.1:3389`, pasta partilhada `/shared`). CI no GitHub Actions `windows-latest` a confirmar em W10 | 18/11/2026 |
| W5 | Stack | **Tauri 2 + Svelte 5**, como o lx (L1). No Windows a janela usa o WebView2 (já vem no Windows 11), o que afasta o problema do WebKitGTK com NVIDIA | 18/11/2026 |
| W6 | Formato | **Continua a ser v2.** Os campos de Windows entram como campos opcionais ao abrigo da secção 4.7 do `FORMAT-v2.md` (não referem blobs, logo não precisam de versão nova). Adenda escrita e aprovada antes do código (4.3) | 18/11/2026 |
| W7 | Automático | **Opcional**, como L3: por omissão só lembra. Quem quiser liga uma tarefa do Agendador de Tarefas do utilizador (sem administrador) que corre `xkyhold-wd.exe --scheduled` de hora a hora | 18/11/2026 |
| W8 | Destinos | **Google Drive + pasta local**, como L4. A pasta local aceita caminhos UNC (`\\nas\backups`). **Cliente Google próprio "XkyHold-wd"** no projeto `xkyhold` (4.4) | 18/11/2026 |
| W9 | Segredos | **Gestor de Credenciais do Windows** (Credential Manager, por DPAPI), atrás da mesma interface `SecretStore` do crate `secrets` | 18/11/2026 |
| W10 | Distribuição | **Instalador NSIS por utilizador** (instala em `%LOCALAPPDATA%`, sem administrador), `.msi` opcional, nas releases do GitHub; avisos de atualização com assinatura própria, como o lx (`docs/UPDATES.md`). **Sem assinatura Authenticode:** o SmartScreen avisa "editor desconhecido" nas primeiras instalações; a página de download explica como continuar. winget depois da 1.0 | 18/11/2026 |
| W11 | Interface | **Copiar a `ui/` do lx** e extrair o que for comum depois da 1.0 (4.7) | 18/11/2026 |

Herdadas da família e mantidas: cifra no cliente e sem servidor; "apagar" no Drive é sempre mover para o lixo; chave de recuperação; EN + pt-PT com "tu"; identidade visual da XkyHold (verde-água, layout neutro e calmo, claro/escuro); sem estatísticas nem telemetria; licença AGPL-3.0-only; a app chama-se "o XkyHold-wd" em português.

## 3. Limitações que o wd resolve

| Limitação | Na web | No wd |
|---|---|---|
| Pasta do utilizador, Documentos, Ambiente de trabalho | O Chrome recusa-as | Raiz por omissão = `%USERPROFILE%` |
| `AppData`, `.ssh`, ficheiros ocultos e de sistema | Recusados ou invisíveis | Guardados (sempre cifrados) |
| Atributos (oculto, só de leitura), data de criação | Não expostos ao browser | Guardados e repostos |
| Data de modificação no restauro | Perdida | Reposta (precisão de 100 ns do NTFS) |
| Ligações simbólicas e junctions | Não expostas | Guardadas como ligações, nunca seguidas |
| Caminhos com mais de 260 caracteres | — | Lidos e escritos com o prefixo `\\?\` |
| Sessão Google expira ao fim de 1 hora | Sem refresh token | Cliente OAuth de app instalada com refresh token |
| Sem cópia com o browser fechado | Sem processo residente | Agendador de Tarefas + binário sem janela |
| Só Chrome/Edge | API do browser | Não depende do browser |

O lx já resolve tudo isto em Linux; o wd traz o mesmo para Windows.

## 4. Arquitetura

### 4.1 Repositórios (W1)

- **xkyhold-lx** (público): os crates do motor passam a compilar e a testar em Windows. O trabalho faz-se num ramo próprio desse repositório; o CI do lx ganha uma linha `windows-latest` para os crates partilhados. Push e merge no lx só com o "sim" do proprietário, como sempre.
- **xkyhold-wd** (este): workspace Cargo com o binário da janela (`src-tauri`), o CLI (`xkyhold-wd-cli`), a `ui/` e o que for só de Windows (agendador, scan da pasta do utilizador, instalador). Depende dos crates do lx por `git = "…xkyhold-lx"`, `rev = "…"` fixa; em desenvolvimento local, `[patch]` para o checkout irmão.
- Se mais tarde se quiser extrair o motor para um repositório `xkyhold-engine`, faz-se sem mexer no formato. Não é preciso agora.

Ponto de partida conhecido: em `xkyhold-lx/crates`, só `core/src/backup.rs` e `core/src/restore.rs` (e, no crate `app`, `lib.rs`, `scan.rs`, `update.rs`) usam APIs de Unix. `dest-local` e `dest-drive` não usam.

### 4.2 Motor em Windows

- **Percurso:** `FindFirstFileExW`/`GetFileInformationByHandleEx` (ou `std::fs::symlink_metadata` + `std::os::windows`), sem seguir reparse points. Entram ficheiros normais, pastas, ligações simbólicas e junctions. Outros reparse points (deduplicação, ficheiros na nuvem) tratados à parte (4.6).
- **Ficheiros bloqueados (W3):** abrir com `FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE`; se mesmo assim falhar por partilha (`ERROR_SHARING_VIOLATION`, `ERROR_LOCK_VIOLATION`), saltar e contar em `stats.locked`, com a lista para o ecrã.
- **Deteção de alterações** (campo `change`): `ChangeTime` + file index (`FileId`) em vez de `ctime` + inode.
- **Restauro seguro:** nomes validados também contra as regras do Windows (`< > : " | ? *`, nomes reservados `CON`, `NUL`, `COM1`…, ponto ou espaço final). Um nome que não se pode criar em Windows (vindo de uma cópia Linux) é restaurado com um nome seguro e reportado. Escrita com nome temporário e renomeação; nunca sair da pasta de destino (verificar reparse points a cada nível).
- **Maiúsculas/minúsculas:** o NTFS não distingue `A.txt` de `a.txt`. Restaurar uma cópia Linux com os dois: o segundo vai para o lado com nome seguro, reportado.
- **Locks locais:** mutex nomeado (`Local\xkyhold-wd`) em vez de `flock`.

### 4.3 Formato (W6)

A especificação escreve-se antes do código, como adenda ao `FORMAT-v2.md` no repositório `xkyhold`, e o proprietário aprova-a. Direção:

- Campos opcionais novos nas entradas: `winAttrs` (lista de atributos guardados: `readonly`, `hidden`, `system`, `archive`, `notIndexed`), `btime` + `btimeNanos` (data de criação), `reparse` só para mostrar.
- `mtime` + `mtimeNanos` continuam a ser a fonte da data de modificação (o NTFS tem 100 ns: cabe em `mtimeNanos`).
- `mode`, `uid`, `gid`, `xattrs` não são escritos pelo wd. Ao restaurar em Windows uma cópia do lx, são ignorados (o bit de execução perde-se, como na web); ao restaurar no lx uma cópia do wd, ficam os modos por omissão.
- Junctions guardadas como `symlink` com o alvo; o restauro em Linux cria uma ligação simbólica. Alvos absolutos de outro sistema (`C:\…`) são restaurados tal como estão e marcados como "aponta para outro sistema".
- O snapshot indica o sistema de origem (`source.os: "windows"`) e o separador, para o restauro "para o sítio original".
- **Fora da v2 do wd:** Alternate Data Streams (exigiriam referências a blobs → versão nova), ACLs/descritores de segurança (o restauro herda as permissões da pasta de destino, o que é o comportamento esperado para dados de utilizador).
- Fixtures novas `golden-v2-win`, criadas uma vez e congeladas com a adenda. Os `golden-v1*` e `golden-v2-repo` nunca se regeneram.
- Testes cruzados: cópia do wd restaurada pelo lx e pela web; cópia do lx restaurada pelo wd.

### 4.4 Google Drive

Mesmo projeto Google Cloud `xkyhold` e scope `drive.file`, para que as três apps vejam as cópias umas das outras. **Cliente Desktop próprio "XkyHold-wd"** (W8), criado na consola só com o "sim" do proprietário para esse ato; o ID e o segredo vão para `.env.local`, fora do git. A conta Workspace onde o administrador autorizou o cliente lx teria de autorizar também o novo.

### 4.5 Estado local

| O quê | Onde |
|---|---|
| Chaves do repositório e refresh token | Gestor de Credenciais (W9). Sem ele: palavra-passe em cada cópia e cópia automática indisponível |
| Configuração | `%APPDATA%\xkyhold-wd\config.toml` |
| Cache do índice, registos | `%LOCALAPPDATA%\xkyhold-wd\` |

As pastas da própria app ficam sempre fora da cópia. A palavra-passe nunca é guardada.

### 4.6 O que guardar e o que deixar de fora

- Por omissão: `%USERPROFILE%` inteira; em alternativa, pastas e ficheiros escolhidos dentro dela.
- Sempre de fora: `AppData\Local\Temp`, a Reciclagem, `NTUSER.DAT*` e `UsrClass.dat*` (sempre bloqueados), pastas com `CACHEDIR.TAG`, o estado da própria app.
- **OneDrive e ficheiros na nuvem:** ficheiros "só online" (atributo `RECALL_ON_DATA_ACCESS`/`OFFLINE`) **nunca são lidos**, porque lê-los obriga o Windows a descarregá-los. Guardados como entradas sem conteúdo? Não: saltados e contados em `stats.cloudOnly`. A pasta do OneDrive aparece como candidata a ficar de fora, como o Dropbox no lx.
- Sugestões desligadas por omissão, com o tamanho: `node_modules`, `target`, `.venv`, caches de browsers (`AppData\Local\Google\Chrome\User Data\*\Cache`, Edge, Firefox), `AppData\Local\Packages\*\LocalCache`, `AppData\Local\Docker`, discos de WSL (`ext4.vhdx`, também bloqueados enquanto o WSL corre), bibliotecas Steam, imagens de VMs, `Downloads`.
- Pontos de montagem e junctions para fora da pasta do utilizador: guardados como ligação, nunca atravessados (W2).

### 4.7 Interface

A `ui/` do lx com os mesmos ecrãs (Assistente, Início, Restaurar, Definições), tokens de cor e traduções. Diferenças: caminhos com `\`, nomes das pastas do Windows, ecrã de "ficheiros bloqueados" (W3) e de "só online" (4.6), e as Definições ligam a tarefa do Agendador em vez do temporizador systemd. Decidido (W11): copiar a `ui/` do lx e extrair o que for comum depois da 1.0, para não travar o lx.

### 4.8 Cópia automática (W7)

- Tarefa `\XkyHold\xkyhold-wd` no Agendador, do utilizador atual, "executar só quando o utilizador tem sessão iniciada", sem privilégios elevados, a cada hora e "executar logo que possível depois de um início falhado" (equivalente ao `Persistent=true`).
- Corre `xkyhold-wd.exe --scheduled` (subsistema Windows, sem janela de consola): cópia automática se em atraso, senão lembrete, como no lx.
- Notificações Windows (toast) só em problemas, uma por dia.
- Ao desinstalar, o instalador remove a tarefa.

### 4.9 Distribuição (W10)

Instalador NSIS por utilizador (Tauri), sem assinatura Authenticode (W10), WebView2 com o modo "bootstrapper" para máquinas sem ele. Releases no GitHub com `.sig`, `latest.json` e `SHA256SUMS`, como o lx. Compilação: no CI `windows-latest` (o mais fiável para MSVC) ou em Linux com `cargo-xwin` (a avaliar na Fase 0). O cliente Google entra por variável de ambiente, fora do git.

## 5. Fases

| Fase | Conteúdo | Feito quando |
|---|---|---|
| **0. Fundação e riscos** | `git init`, esqueleto, `CLAUDE.md`, `README.md`, `LICENSE`. Ferramentas na VM (Rust MSVC, Node, Build Tools) ou caminho de compilação cruzada. Protótipos: (a) janela Tauri abre na VM Windows 11 e chama o Rust; (b) os crates `core`/`dest-*` do lx compilam para `x86_64-pc-windows-msvc` (lista do que falta); (c) login Google com cliente Desktop a partir do Windows (browser da VM), refresh token no Gestor de Credenciais; (d) leitura de um ficheiro bloqueado e de um ficheiro "só online" do OneDrive, para confirmar como o Windows responde | Os quatro riscos com resposta escrita na secção 7 |
| **1. Adenda ao formato** | Escrever a adenda Windows ao `FORMAT-v2.md`; rever com o proprietário | Adenda aprovada |
| **2. Motor multiplataforma** | No ramo do lx: percurso, restauro seguro, `change`, locks e scan em Windows; CI do lx com `windows-latest`; `golden-v2-win`; testes cruzados | Cópia e restauro de uma árvore de teste com atributos, datas, junctions, nomes longos e nomes impossíveis dão o resultado esperado na VM; os testes do lx continuam verdes em Linux |
| **3. Google Drive e segredos** | Cliente OAuth (se W8 = próprio), Credential Manager, CLI `xkyhold-wd-cli` | Cópia e restauro reais no Drive pelo CLI na VM, incluindo uma cópia de mais de 1 hora |
| **4. Interface** | Janela Tauri: Assistente, Início, Restaurar, Definições, EN + pt-PT, claro/escuro | Fluxo completo pela janela na VM, com o Drive e com pasta local |
| **5. Automático** | Tarefa do Agendador, mutex, toasts, lembrete | Cópia automática vista a correr com a janela fechada e depois de reiniciar a VM |
| **6. Leitura cruzada** | A web e o lx restauram cópias do wd (e o wd as deles) | Cópia do wd aberta na web (pré-visualização local) e no lx |
| **7. Prova real e release** | Cópia de uma pasta de utilizador realista na VM, verificar, restauro completo e comparação; restauro noutro utilizador Windows limpo; instalador; página `/windows/` no site (terceiro cartão na grelha `.ways`, caso Windows no `landing-os.js`, palavra na stack do xkyriver.com — `xkyhold/docs/LANDING.md`) | v1.0.0 publicada |

## 6. Regras de trabalho

As do lx, sem exceção (secção 6 do `xkyhold-lx/docs/PLAN.md` e secção 3 do checkpoint), mais:

- **A VM é para testes:** nada de contas ou dados reais do proprietário lá dentro, exceto o login Google que ele próprio fizer. Os repositórios de teste no Drive chamam-se `XkyHold-wd test…`.
- **O repositório do lx é público e está lançado:** as mudanças ao motor vão num ramo; nenhuma muda o comportamento do lx sem testes verdes em Linux e sem o "sim" do proprietário para o merge e o push.
- Passos que dependem do proprietário (login na VM, diálogos do Windows, palavras-passe): um de cada vez, com aviso antes.

## 7. Registo do estado

- 18/11/2026: plano base escrito. Decisões W1–W4 tomadas pelo proprietário; W5–W10 propostas e aprovadas no mesmo dia, mais W11 (interface). A VM Windows 11 (`omarchy-windows`) está instalada e a correr. Nada implementado; a pasta ainda não é um repositório git.

### Fase 0 (18–19/11/2026, concluída)

Feito:

- `git init` local (ramo `main`, identidade `xkyriver` neste repositório), sem remoto. `LICENSE` (AGPL-3.0-only, o mesmo texto), `.gitignore`, `mise.toml` e `rust-toolchain.toml` do lx (este com o alvo `x86_64-pc-windows-msvc`), `CLAUDE.md`, `README.md`.
- Alvo Rust `x86_64-pc-windows-msvc` instalado no toolchain 1.100.0 do mise (só a biblioteca padrão; nada de Microsoft).

Risco (b), os crates do lx compilam para Windows? Resposta com `cargo check --target x86_64-pc-windows-msvc` numa cópia descartável do workspace do lx (o repositório do lx não foi tocado):

- Tal como estão, nenhum crate que dependa do `core` compila, porque o `core` puxa a `uzers` (só Unix). O `xkyhold-secrets` compila sem erros.
- Com `rustix` e `uzers` passados para `[target.'cfg(unix)'.dependencies]`, **todos os 27 erros ficam em `core/src/backup.rs` e `core/src/restore.rs`** (descritores, `openat`, `OsStr::as_bytes`, dono/grupo). O resto do `core` (cifra, packs, índice, árvores, snapshots, regras, prune, locks, check) compila para Windows sem mudanças. As dependências de cifra, o `ureq` e o TLS também.
- Consequência para a Fase 2: o trabalho no motor resume-se a um percurso e a um restauro por plataforma (`backup/unix.rs` + `backup/windows.rs`, idem no restauro), com o que é comum (incremental, packs, árvores) partilhado. No crate `app` do lx, `lib.rs`, `scan.rs` e `update.rs` têm partes Unix; o wd terá o seu próprio `scan`, agendador e atualizador.
- Não testado: compilar e ligar (`link.exe`) de facto, o que só se faz na VM ou no CI; o crate `dest-local` e o `dest-drive` em Windows (não usam APIs Unix, mas só compilam depois do `core`).

VM de testes preparada (18/11/2026):

- Acesso por SSH a partir do anfitrião: o contentor encaminha todas as portas para o Windows exceto as dele, por isso basta o IP do contentor (`172.19.0.2`, rede `windows_default`), porta 22. Chave própria só para a VM, sem palavra-passe; atalho `xkyvm` na configuração SSH do proprietário. O servidor é o pacote `Microsoft.OpenSSH.Preview` do winget, ligado por `scripts/vm/enable-ssh.ps1` (o caminho pelo Windows Update, `Add-WindowsCapability`, ficou mais de 30 minutos sem acabar e foi abandonado). A shell do SSH é o Windows PowerShell 5.1.
- Ferramentas instaladas por `scripts/vm/setup-vm.ps1 -AcceptLicenses` (licenças aceites pelo proprietário a 18/11/2026): Git 2.56.0, Node 26.8.1 (npm 11.19.0), rustup 1.29.3 com Rust 1.100.0 (`rustfmt`, `clippy`), Visual Studio 2022 Build Tools 17.14 (C++ e Windows SDK). WebView2 157 já vinha com o Windows 11 Pro (build 26200).
- Visto: um `cargo new` compilado e ligado com o MSVC na VM corre e imprime "Hello, world!".
- Armadilhas: o `winget` não está no PATH das sessões SSH (é um alias de `WindowsApps`; o script procura o executável); o `sshd` mantém o PATH com que arrancou, por isso cada comando por SSH começa por reler o PATH do registo; a política de execução do PowerShell é `Restricted` (perfis não carregam; scripts correm com `-ExecutionPolicy Bypass`).

Risco (a), janela Tauri no Windows (18/11/2026): **funciona sem ajustes.** Esqueleto mínimo neste repositório (`src-tauri` com Tauri 2.12 e um comando `probe`; `ui` em Svelte 5 + Vite 8 na porta 5175, com o `app.css` do lx; instalador NSIS por utilizador configurado mas ainda não gerado). Compilado na VM com `npx.cmd tauri build --debug --no-bundle` (primeira compilação 6 min 44 s). Visto pelo agente em captura do ecrã da VM: a janela abre com o WebView2, mostra o desenho da XkyHold e a resposta do Rust (`windows (x86_64)`, `C:\Users\xriver`, `0.1.0`). Atrás dela fica a consola, como esperado num build de debug (o `windows_subsystem = "windows"` só se aplica em release). Não testado: build de release, instalador NSIS, tema escuro, ecrãs com escala acima de 100 %.

Como ver a VM a partir do SSH: `scripts/vm/in-desktop.ps1` corre um programa na sessão do ambiente de trabalho por uma tarefa agendada de logon interativo (sem palavra-passe; nome único por chamada, porque uma tarefa ainda a correr recusa um segundo arranque) e `scripts/vm/screenshot.ps1` grava uma captura. Armadilhas: com a compilação a ocupar as 4 CPUs, as tarefas no ambiente de trabalho ficaram paradas; o `npm`/`npx` do PowerShell são scripts `.ps1` bloqueados pela política `Restricted`, por isso usam-se `npm.cmd`/`npx.cmd`.

Risco (d), ficheiros bloqueados e "só online" (18/11/2026). Protótipo `crates/probe-files` (lê cada ficheiro como a cópia o fará: `File::open` do Rust, que no Windows pede partilha de leitura, escrita e remoção) contra ficheiros de teste na VM seguros por `scripts/vm/hold-locks.ps1` noutro processo (`scripts/vm/probe-files.ps1` junta tudo). Visto pelo agente na VM:

| Caso | Resposta do Windows |
|---|---|
| Ficheiro aberto por outro programa sem partilha (`FileShare.None`) | `open` falha com o erro 32 (`ERROR_SHARING_VIOLATION`) |
| Ficheiro aberto para escrita com partilha de leitura | Lê-se normalmente |
| Ficheiro com o conteúdo trancado por intervalo (`LockFileEx`, como fazem bases de dados e o Outlook) | `open` funciona; o `read` falha com o erro 33 (`ERROR_LOCK_VIOLATION`) logo no primeiro byte |
| `NTUSER.DAT` e `UsrClass.dat` | Erro 32, sempre |
| Ocultos e de sistema | Lêem-se normalmente; os atributos aparecem em `file_attributes()` |
| Atributo `Offline` posto à mão | Lê-se normalmente (é só uma marca; sem fornecedor de nuvem nada é descarregado) |

Consequências para a Fase 2: os erros 32 e 33 contam como "bloqueado" (`stats.locked`, com a lista para o ecrã), também quando o 33 aparece a meio da leitura (nunca se guarda um ficheiro parcial); `NTUSER.DAT*` e `UsrClass.dat*` ficam de fora por omissão, como previsto em 4.6; os atributos lêem-se sem `unsafe`, com `std::os::windows::fs::MetadataExt`.

Ficheiro "só online" real (19/11/2026), com o OneDrive da VM ligado à conta do proprietário (login dele). Dois ficheiros de 2 MiB criados numa pasta de teste `OneDrive\XkyHold-wd probe`, sincronizados e postos "só online" com `attrib -P +U`. Visto pelo agente:

| Passo | Resultado |
|---|---|
| Atributos pelo Explorador/PowerShell | `0x501620`: `RecallOnDataAccess`, `Unpinned`, `Offline`, `ReparsePoint`, `Sparse` |
| Atributos pelo Rust (`symlink_metadata`) | `0x500020`: só `RecallOnDataAccess` e `Unpinned`. O Windows esconde o reparse point, o `Offline` e o `Sparse` de processos que não declaram conhecer ficheiros na nuvem; o `RecallOnDataAccess` continua visível e chega para os reconhecer |
| Ler só os metadados | Nada descarregado; atributos iguais |
| Ler o conteúdo a partir do SSH (sessão 0, administrador) | Falha com o erro 395 (`ERROR_CLOUD_FILE_ACCESS_DENIED`) sem descarregar |
| Ler o conteúdo a partir do ambiente de trabalho (utilizador normal, como a app) | **Descarrega**: 2 MiB lidos em 1,6 s, e o ficheiro passa a `0x420` (já local). O outro ficheiro, não lido, continuou "só online" |

Conclusão: a regra de 4.6 está certa e é necessária. O percurso tem de decidir **antes de abrir** o ficheiro, pelo `RecallOnDataAccess` (e `RecallOnOpen`) dos metadados, e contar o ficheiro em `stats.cloudOnly`. O erro 395 também conta como "só online". Os ficheiros já descarregados (`0x420`) lêem-se como os outros. A pasta de teste foi para a Reciclagem (fica também no lixo do OneDrive); nada mais da conta foi tocado.

Cliente Google "XkyHold-wd" (19/11/2026): tipo Desktop app, criado pelo agente na consola do projeto `xkyhold` (Chrome "pulsar" do proprietário), com o "sim" dele. ID `780229057994-0583pofk3onqt68ijqlip77ahvi5julr.apps.googleusercontent.com`. O segredo só existe no `.env.local` (ignorado pelo git, permissões 600; copiado pelo proprietário, o agente não o viu). O `.env.example` mostra as variáveis `XKYHOLD_WD_CLIENT_ID` e `XKYHOLD_WD_CLIENT_SECRET`.

Risco (c), login Google a partir do Windows (19/11/2026): **funciona.** Protótipo `crates/probe-oauth` (PKCE + redirecionamento para `127.0.0.1` numa porta livre, browser por omissão aberto com `rundll32 url.dll,FileProtocolHandler`, refresh token no Gestor de Credenciais por `keyring-core` + `windows-native-keyring-store`), corrido na sessão do ambiente de trabalho da VM por `scripts/vm/probe-oauth.ps1`, com o ID e o segredo copiados do `.env.local` para um ficheiro temporário que se apaga no fim. Login do proprietário no Edge da VM, com a conta pessoal. Visto pelo agente no registo do protótipo:

- a Google concedeu só `drive.file`;
- refresh token recebido, guardado no Gestor de Credenciais (entrada genérica `google-refresh:probe.xkyhold-wd`, vista com `cmdkey` na sessão do utilizador), lido de volta igual, e usado para obter um token novo válido por 1 hora;
- o cliente novo vê a pasta `XkyHold` da raiz do Drive, criada por outro cliente do mesmo projeto. A pasta não tinha subpastas visíveis (as de teste do lx foram para o lixo a 14/11);
- `--forget` apagou a entrada; `cmdkey` deixou de a mostrar.

Notas: as credenciais guardadas na sessão interativa não aparecem numa sessão SSH (logon de rede), por isso a verificação foi feita na sessão do ambiente de trabalho. O token não foi revogado na Google: continua listado nas apps com acesso à conta até o proprietário o retirar ou até expirar por falta de uso.

**Fase 0 concluída a 19/11/2026:** os quatro riscos têm resposta. Próxima: Fase 1, adenda Windows ao `FORMAT-v2.md` no repositório `xkyhold`, para aprovação do proprietário.


## 8. Questões em aberto

Nenhuma de momento. W5–W11 aprovadas a 18/11/2026.
