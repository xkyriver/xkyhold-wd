# XkyHold-wd — plano base

Data: 18/11/2026 · Estado: **plano aprovado (decisões W1–W11, 18/11/2026); Fase 0 concluída a 19/11/2026; Fases 1, 2 e 3 concluídas a 20/11/2026, Fases 4, 5 e 6 a 21/11/2026; Fase 7 (prova real e release) em curso**

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
| W1 | Código | **Motor partilhado, num repositório próprio `xkyhold-engine`** (crates `core`, `dest-local`, `dest-drive`, `secrets`, extraídos do xkyhold-lx 1.0.2 com o histórico). Passa a multiplataforma (`cfg(windows)` / `cfg(unix)`); o xkyhold-wd depende dele por git com rev fixa. O xkyhold-lx não é tocado: fica com a sua cópia congelada até o proprietário decidir passá-lo para o `xkyhold-engine`. Um só motor a evoluir | 18/11/2026; repositório próprio decidido a 20/11/2026 (substitui o ramo no lx) |
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

- **xkyhold-engine** (novo, 20/11/2026; repositório **público** `xkyriver/xkyhold-engine` desde o mesmo dia): workspace com `crates/core`, `crates/dest-local`, `crates/dest-drive`, `crates/secrets`, extraídos do xkyhold-lx 1.0.2 com o histórico (`git filter-branch` num clone), mais `scripts/cross-ts*`, CI (Linux; Windows entra na Fase 2), `README.md`, `CLAUDE.md`. Os 49 testes do lx passam lá. É aqui que se faz o trabalho multiplataforma. 
- **xkyhold-wd** (este): workspace Cargo com o binário da janela (`src-tauri`), o CLI (`xkyhold-wd-cli`), a `ui/` e o que for só de Windows (agendador, scan da pasta do utilizador, instalador). Depende dos crates do engine por `git = "…xkyhold-engine"`, `rev = "…"` fixa; em desenvolvimento local, `[patch]` para o checkout irmão.
- **xkyhold-lx** (público): não é tocado. Continua com a sua cópia do motor (1.0.2). Passá-lo a depender do `xkyhold-engine` é uma decisão futura do proprietário; até lá, uma correção ao motor que interesse ao lx tem de ser levada lá à mão.

Ponto de partida conhecido (no engine, como estava no lx): só `core/src/backup.rs` e `core/src/restore.rs` (e, no crate `app`, `lib.rs`, `scan.rs`, `update.rs`) usam APIs de Unix. `dest-local` e `dest-drive` não usam.

### 4.2 Motor em Windows

- **Percurso:** `FindFirstFileExW`/`GetFileInformationByHandleEx` (ou `std::fs::symlink_metadata` + `std::os::windows`), sem seguir reparse points. Entram ficheiros normais, pastas, ligações simbólicas e junctions. Outros reparse points (deduplicação, ficheiros na nuvem) tratados à parte (4.6).
- **Ficheiros bloqueados (W3):** abrir com `FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE`; se mesmo assim falhar por partilha (`ERROR_SHARING_VIOLATION`, `ERROR_LOCK_VIOLATION`), saltar e contar em `stats.locked`, com a lista para o ecrã.
- **Deteção de alterações** (campo `change`): `ChangeTime` + file index (`FileId`) em vez de `ctime` + inode.
- **Restauro seguro:** nomes validados também contra as regras do Windows (`< > : " | ? *`, nomes reservados `CON`, `NUL`, `COM1`…, ponto ou espaço final). Um nome que não se pode criar em Windows (vindo de uma cópia Linux) é restaurado com um nome seguro e reportado. Escrita com nome temporário e renomeação; nunca sair da pasta de destino (verificar reparse points a cada nível).
- **Maiúsculas/minúsculas:** o NTFS não distingue `A.txt` de `a.txt`. Restaurar uma cópia Linux com os dois: o segundo vai para o lado com nome seguro, reportado.
- **Locks locais:** mutex nomeado (`Local\xkyhold-wd`) em vez de `flock`.

### 4.3 Formato (W6)

> Substituída pela adenda aprovada `docs/FORMAT-v2-windows.md` do repositório `xkyhold` (Fase 1, 20/11/2026). O texto abaixo é a direção inicial.

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

- Tarefa `\XkyHold\xkyhold-wd-<utilizador>` no Agendador (o nome leva o utilizador do Windows: os nomes das tarefas são do computador inteiro; visto na Fase 7 com dois utilizadores), do utilizador atual, "executar só quando o utilizador tem sessão iniciada", sem privilégios elevados, a cada hora e "executar logo que possível depois de um início falhado" (equivalente ao `Persistent=true`).
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
| **2. Motor multiplataforma** | No `xkyhold-engine`: percurso, restauro seguro, `change`, locks e scan em Windows; CI com `windows-latest`; `golden-v2-windows-repo.json`; testes cruzados | Cópia e restauro de uma árvore de teste com atributos, datas, junctions, nomes longos e nomes impossíveis dão o resultado esperado na VM; os testes do engine continuam verdes em Linux |
| **3. Google Drive e segredos** | Cliente OAuth (se W8 = próprio), Credential Manager, CLI `xkyhold-wd-cli` | Cópia e restauro reais no Drive pelo CLI na VM, incluindo uma cópia de mais de 1 hora |
| **4. Interface** | Janela Tauri: Assistente, Início, Restaurar, Definições, EN + pt-PT, claro/escuro | Fluxo completo pela janela na VM, com o Drive e com pasta local |
| **5. Automático** | Tarefa do Agendador, mutex, toasts, lembrete | Cópia automática vista a correr com a janela fechada e depois de reiniciar a VM |
| **6. Leitura cruzada** | A web e o lx restauram cópias do wd (e o wd as deles) | Cópia do wd aberta na web (pré-visualização local) e no lx |
| **7. Prova real e release** | Cópia de uma pasta de utilizador realista na VM, verificar, restauro completo e comparação; restauro noutro utilizador Windows limpo; instalador; página `/windows/` no site (terceiro cartão na grelha `.ways`, caso Windows no `landing-os.js`, palavra na stack do xkyriver.com — `xkyhold/docs/LANDING.md`) | v1.0.0 publicada |

## 6. Regras de trabalho

As do lx, sem exceção (secção 6 do `xkyhold-lx/docs/PLAN.md` e secção 3 do checkpoint), mais:

- **A VM é para testes:** nada de contas ou dados reais do proprietário lá dentro, exceto o login Google que ele próprio fizer. Os repositórios de teste no Drive chamam-se `XkyHold-wd test…`.
- **O motor vive no `xkyhold-engine`:** o repositório do lx (público, lançado) não é tocado neste projeto. Os testes do engine têm de continuar verdes em Linux e em Windows.
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


### Fase 1 (20/11/2026, concluída)

- Adenda `docs/FORMAT-v2-windows.md` no repositório `xkyhold`: commits `a9995c9` e `abb5890`, juntados ao `main` (avanço direto) e enviados para o GitHub a 20/11/2026 com o "sim" do proprietário (só documentação; sem deploy). **Aprovada e congelada pelo proprietário a 20/11/2026.** Não é uma versão nova do formato: só campos opcionais (secção 4.7 da v2) e regras sobre os campos que um escritor Windows preenche.
- Lido no código dos leitores já lançados: o lx 1.0.x exige `uid`/`gid` dentro de `home` e os três campos de `change`; por isso o wd **não escreve `home`** (usa `origin`, novo) e preenche `change` com os mesmos três campos (`ChangeTime`, file ID). Ambos ignoram campos desconhecidos. Nenhum leitor lançado precisa de mudar.
- O que a adenda fixa: `origin` (`os`, `path`, `user`, `sid`); `winAttrs`, `btime` + `btimeNanos`; junctions e ligações simbólicas como `symlink` com `link: file|dir|junction`; `stats.locked`, `stats.cloudOnly`, `stats.badNames`; `leftOut` com os padrões do Windows; regras de restauro no Windows (nomes impossíveis e maiúsculas/minúsculas com nome seguro e lista; `readonly` por último); fixture `golden-v2-windows-repo.json` a escrever uma vez pelo wd. A secção 4.3 deste plano fica substituída pela adenda.
- Decisões do proprietário: **D1** nome UTF-16 inválido não se guarda, conta em `badNames` e **o utilizador é informado** de quais e porquê; **D2** na cópia só as contagens, a lista de caminhos fica no registo local da app; **D3** ligação simbólica que o Windows recusa não se restaura e **o utilizador é informado** (quais, para onde apontavam, e que ligar o Modo de Programador e restaurar de novo as traz); **D4** data de criação guardada e reposta.

Repositórios GitHub criados a 20/11/2026 com o "sim" do proprietário: `xkyriver/xkyhold-engine` (público) e `xkyriver/xkyhold-wd` (**privado** até à release).

Próxima: Fase 2, motor multiplataforma no repositório `xkyhold-engine` (decisão do proprietário: substitui o ramo no lx; o lx não é tocado).

### Fase 2 (20/11/2026, concluída)

No `xkyhold-engine` (commits `1710b6a` a `b76db92`):

- Tipos: `Snapshot.origin`, `Stats.locked/cloudOnly/badNames`, `Meta.winAttrs/btime/btimeNanos`, `link` nas ligações. Os `golden` antigos continuam iguais.
- `backup/` e `restore/` com parte comum + `unix.rs` (comportamento igual ao do lx) + `windows.rs`, segundo a adenda. `rustix`/`uzers` só em Unix. Caminhos `\\?\` na cópia e no restauro (mais de 260 caracteres).
- **Exceção ao `unsafe`** (decisão do proprietário, 20/11/2026): só em `win.rs`, com `windows-sys` da Microsoft e cada chamada comentada (`GetFileInformationByHandleEx`, token do processo, `SetFileAttributesW`, `DeviceIoControl` para junctions).
- **Elevação** (decisão do proprietário, 20/11/2026): recusa só "Executar como administrador" com o UAC ligado (`TokenElevationTypeFull`); com o UAC desligado funciona; nunca pede elevação. A VM de testes tem o UAC desligado.
- `secrets`: Gestor de Credenciais no Windows (serviço `xkyhold-wd`), Secret Service em Linux.
- Exclusões por omissão do Windows, sem distinguir maiúsculas; pasta local sem `fsync` de pastas no Windows.
- **`golden-v2-windows-repo.json` congelado** (gerado uma vez na VM com a pasta neutra `C:\Golden\ana` e o utilizador `ana`). A secção 9 da adenda foi clarificada no `xkyhold` (commit `fc5cae6`, por enviar): `cloudOnly` e `special` não se conseguem numa pasta temporária e são testados à parte contra o OneDrive e o `WindowsApps` reais (decisão do proprietário).

Visto pelo agente (testes a passar):

| Onde | Resultado |
|---|---|
| Linux | 51 testes, incluindo ler e restaurar a fixture Windows |
| Windows (VM, sessão do ambiente de trabalho) | 50 testes: ida e volta Windows (atributos, datas de modificação e criação, junction, bloqueado contado e ausente, exclusões), cópia sem alterações que não relê nada, nomes vindos de Linux renomeados e listados, caminho com mais de 300 caracteres, Gestor de Credenciais, fixture Linux restaurada no Windows, fixture Windows lida e restaurada |
| Motor TypeScript da web | Lê a fixture Windows byte a byte, com palavra-passe e chave de recuperação (`scripts/cross-ts-windows.sh`) |
| XkyHold-lx 1.0.2, tal como foi lançado | Abre e restaura a fixture Windows sem alterações (num worktree descartável do lx; nada ficou no lx) |
| OneDrive real e `WindowsApps` da VM | Ficheiro "só online" contado e **não descarregado** (atributos iguais antes e depois); aliases de app contados como especiais. Pasta de teste posta na Reciclagem |
| CI do engine | Linux e `windows-latest` verdes no commit `43e66f3` |

Notas: antes de 1970 a biblioteca padrão arredonda as datas para o lado de 1970 (diferença de 100 ns, aceite no teste). Não testado: um disco de rede (`\\servidor\…`) como origem ou destino; ReFS (file ID de 128 bits real); EFS; Desduplicação de Dados.

**Fase 2 concluída a 20/11/2026.** Próxima: Fase 3 (o wd passa a depender do engine por git; Drive e Gestor de Credenciais no CLI `xkyhold-wd-cli`).

### Fase 3 (20/11/2026, concluída)

- O wd usa os crates do engine por git (`xkyhold-engine` `b76db92`; `8b729e9` desde a Fase 4; `f3781f7`, `f393f86` e `88f0b3a` desde 21/11/2026).
- `crates/app` (`xkyhold-wd-app`, o equivalente Windows do `xkyhold-app` do lx): definições em `%APPDATA%\xkyhold-wd\config.toml`; cache, registo (`xkyhold-wd.log`), lock local (`backup.lock`, `File::try_lock`) e a lista dos itens não guardados da última cópia (`last-backup-not-stored.txt`, decisão D2) em `%LOCALAPPDATA%\xkyhold-wd`; Gestor de Credenciais; cliente Google "XkyHold-wd" embutido na compilação (`XKYHOLD_WD_CLIENT_*`); cópia anterior procurada por computador e `origin.path`.
- `crates/cli` (`xkyhold-wd-cli`): os comandos do lx (sem `--cross-mount` nem `check-release`). No fim da cópia diz quantos itens ficaram bloqueados, "só online" ou com nome inválido e onde está a lista; no fim do restauro lista os renomeados (D1) e as ligações recusadas, com o caminho do Modo de Programador (D3).
- Visto na VM: CLI de ponta a ponta numa pasta local (`init`, `backup --only`, `snapshots`, `ls`, `restore`, `check --full`, `status`). Login Google pelo CLI na sessão do ambiente de trabalho, feito pelo proprietário no Edge; refresh token no Gestor de Credenciais. **Drive real, `XkyHold-wd test 1`:** criar, lembrar a chave, cópia de 12,6 MB aberta só com a chave do Gestor de Credenciais, restauro com conteúdo, data e "só de leitura" iguais, `check --full` sem problemas.
- **Cópia longa, `XkyHold-wd test 2`:** 40 ficheiros (419,4 MB) com `--limit-upload 100`, 4196 s (70 min). O token de acesso renovou-se sozinho a meio da cópia (às 23:03 UTC; o anterior tinha vencido) e outra vez no `check` a seguir, que terminou sem problemas (31 packs, 123 pedaços).

**Fase 3 concluída a 20/11/2026.** Próxima: Fase 4 (janela Tauri com o Drive e com pasta local).

### Fase 4 (20–21/11/2026, concluída)

- A `ui/` do lx copiada (W11) e adaptada ao Windows: "pasta de utilizador", Gestor de Credenciais, caminhos `E:\…` e `\\servidor\partilha\…`, restauro por omissão em `%USERPROFILE%\XkyHold-restore\<data hora>`. Sem o atualizador do lx (não faz parte do plano do wd). As cópias automáticas ficam escondidas até à Fase 5 (`scheduler: false`); até lá a janela só lembra.
- D2 na janela: no fim da cópia, as contagens (em uso, só online, nome inválido) e o botão "Ver a lista", que abre `last-backup-not-stored.txt`; o Início mostra quantos itens ficaram de fora na última cópia. D1 e D3: no fim do restauro, quantos voltaram com outro nome e quantas ligações o Windows recusou (com o caminho do Modo de Programador), com os primeiros 50 em "Detalhes" e a lista toda no registo.
- `crates/app/src/scan.rs`: pastas da pasta de utilizador e espaço das sugestões, sem atravessar reparse points e só com metadados (não descarrega ficheiros do OneDrive); pastas de nuvem incluindo "OneDrive - <organização>".
- Visto na VM pela janela (comandada pela sonda de debug `XKYHOLD_WD_PROBE`, `scripts/vm/js.sh`): **Drive**, `XkyHold-wd test 3`: assistente até ao fim, cópia com um ficheiro bloqueado (aviso D2 e lista aberta no Bloco de Notas), segunda cópia com os 5 ficheiros, restauro para pasta nova com conteúdo, data, "só de leitura" e "oculto" iguais, "Abrir pasta" no Explorador, verificação completa sem problemas, reabertura só com a chave do Gestor de Credenciais. **Pasta local** (`C:\xkyvm\ui-local-repo`): assistente, cópia, restauro "onde estavam" com o conflito mostrado e "manter ambos" (`notes (restored).txt`), verificação. pt-PT e tema escuro vistos em captura.
- Corrigido no teste: o ecrã Restaurar pedia a lista de cópias ao Drive a cada atualização do estado (20 s) e, nesse momento, um clique em Restaurar era recusado com a frase errada ("outro computador"). Agora a lista só se recarrega depois de cada tarefa, os comandos esperam um pouco pela cópia aberta quando não há tarefa a correr, e a recusa durante uma tarefa diz que o XkyHold-wd está ocupado.
- Visto depois (21/11/2026): "Remover deste computador" pela janela volta ao assistente, apaga a conta, a cópia e a chave dessa cópia no Gestor de Credenciais e termina a sessão Google (idioma e tema ficam). Entrar no Google pela janela: o Edge da VM já tinha a sessão e a autorização, e a janela recebeu a conta em segundos. Ligar a uma cópia que já existe (`XkyHold-wd test 3`) com a palavra-passe: a última cópia deste computador e as regras de fora vêm da cópia.
- Corrigido nesse teste: uma cópia não regista a escolha "só algumas pastas" (o formato só guarda o que ficou de fora), por isso ligar a uma cópia existente voltava à pasta de utilizador inteira e a primeira cópia ia guardar tudo (parada ao fim de 20 s; o que foi enviado está só na cópia de teste). Agora o assistente, depois de ligar a uma cópia existente, mostra "O que guardar" para confirmar antes da primeira cópia. Visto: escolha guardada e cópia só da pasta escolhida (6 ficheiros). O lx tem a mesma falha (não mexido).
- Por ver: o seletor de pastas do sistema, os avisos D1/D3 com casos reais, ecrãs com escala acima de 100 %.

**Fase 4 concluída a 21/11/2026.** Próxima: Fase 5 (Agendador de Tarefas, toasts, lembrete com a janela fechada). As sugestões a deixar de fora próprias do Windows (secção 4.6) estão no `xkyhold-engine` `8b729e9` (51 testes em Linux e 51 no Windows); vistas na janela com o tamanho real (caches do Edge 10 MB). O wd usa essa revisão desde 21/11/2026 (9 testes do wd passam no Windows).

### Fase 5 (21/11/2026, concluída)

- `crates/app/src/schedule.rs`: a decisão de cada hora (cópia automática se estiver em atraso, senão lembrete, como no lx) e a tarefa do Agendador, registada com `schtasks.exe /Create /XML` (sem dependências novas nem `unsafe`). Tarefa `\XkyHold\xkyhold-wd` (se o Windows não deixar criar a pasta, `\XkyHold-wd`): só com a sessão iniciada (`InteractiveToken`), privilégios mínimos (`LeastPrivilege`), a cada hora com 5 min aleatórios e 10 min depois do início de sessão, "logo que possível depois de um início falhado", também com bateria, sem limite de tempo, nunca duas ao mesmo tempo, prioridade abaixo do normal. Só existe enquanto houver cópia configurada e cópias automáticas ou lembretes ligados.
- `xkyhold-wd.exe --scheduled` corre sem janela. As notificações são toasts mostrados por um PowerShell escondido, com o nome "XkyHold-wd" registado em `HKCU\Software\Classes\AppUserModelId` (sem elevação); problemas e itens deixados de fora no máximo uma vez por dia.
- A janela mantém a tarefa a par das definições (ao abrir, ao ligar uma cópia, ao mudar lembretes ou cópias automáticas, ao esquecer a chave, ao remover o computador) e só aceita cópias automáticas com a chave no Gestor de Credenciais.
- Visto na VM: tarefa criada pelo assistente ("cópia automática todos os dias"); com a janela fechada, a tarefa fez a cópia sozinha; lembrete em pt-PT visto como toast "XkyHold-wd"; **depois de reiniciar a VM**, a tarefa correu 10 min após o início de sessão e fez a cópia em atraso (resultado 0).
- Corrigido no teste: a hora de início ia em UTC sem `Z` e o Windows lia-a como hora local (8 h de atraso na VM).
- Armadilha da VM: o relógio está 8 h adiantado e o `cargo` não recompilava crates copiados por `tar`; extrair com `tar -xmzf`. Os resultados anteriores na VM foram revistos e mantêm-se.
- Por ver: um utilizador Windows padrão (sem administrador) a criar a pasta `\XkyHold` no Agendador (a VM corre como administrador com UAC desligado); o build de release (sem consola); o instalador a remover a tarefa ao desinstalar (Fase 7).

**Fase 5 concluída a 21/11/2026.** Próxima: Fase 6 (leitura cruzada com a web e o lx).

### Fase 6 (21/11/2026, concluída)

Leitura cruzada vista com cópias reais, nos quatro sentidos:

| Sentido | Como | Resultado |
|---|---|---|
| wd → lx | Cópia local feita pela janela do wd na VM (`auto-repo`: nomes com acentos, só de leitura, oculto, junction, 3 MB), lida pelo **lx 1.0.2** publicado (CLI, numa worktree descartável) | `snapshots`, `ls`, restauro e `check --full` sem problemas; conteúdo igual (checksum); a junction volta como ligação com o alvo Windows e o "só de leitura" não se aplica, como diz `FORMAT-v2-windows.md` 6 |
| lx → wd | Cópia do lx 1.0.2 de uma pasta Linux de teste (`a:b.txt`, `what?.txt`, `CON.txt`, `fim.`, ligação para ficheiro e para pasta, 2 MB, data de 2020), aberta pela janela do wd | Os 6 ficheiros iguais, data de 2020 mantida; D1 visto: 4 nomes renomeados (`a_b.txt`, `what_.txt`, `CON_.txt`, `fim_`) e listados em "Detalhes"; "onde estavam" não oferecido (cópia de Linux); as ligações foram criadas (o utilizador da VM é administrador, por isso a D3 não apareceu) |
| wd → web | Cópia `XkyHold-wd test 4` no Drive feita pela janela do wd, aberta pela XkyHold web em pré-visualização local (`localhost:5173`) | Listada, aberta com a palavra-passe, conteúdo com nomes e tamanhos certos (a junction como "ligação"), verificação "5 ficheiros … estão todos intactos" |
| web → wd | Cópia `XkyHold-wd web test` feita pela web no Chrome do proprietário (formato 1), aberta pela janela do wd | Aberta só para restaurar, como deve; os 3 ficheiros iguais (checksum), nome com acentos e data de 2021 mantidos |

Encontrado pelo caminho (sem alteração nesta fase):
- **Ligações relativas de Linux com `/`** (por exemplo `projeto/notas.md`) são restauradas no Windows tal como guardadas, como manda `FORMAT-v2-windows.md` 5, mas o Windows não as segue. Trocar `/` por `\` nos alvos relativos resolveria; muda o texto da adenda. Decisão do proprietário (secção 8).
- **XkyHold web:** (a) no navegador embutido da app Claude (Electron) o seletor de pastas devolve "cancelado" e o "Permitir" não faz nada; no Chrome funciona; a web não consegue distinguir este caso de um cancelamento; (b) o aviso "1 pasta adicionada" fica no ecrã depois de remover a pasta; (c) numa cópia do wd, as Definições dizem "A palavra-passe desta cópia muda-se no XkyHold-lx". Notas para o repositório `xkyhold`.
- No Drive ficam as cópias de teste `XkyHold-wd test 4` e `XkyHold-wd web test` (para o lixo com o sim do proprietário).

**Fase 6 concluída a 21/11/2026.** Próxima: Fase 7 (prova real e release).

### Fase 7 (em curso, 21/11/2026)

Decisões do proprietário (21/11/2026): o repositório `xkyhold-wd` passa a **público** na release (como o lx e o engine; o "sim" para o ato concreto fica para essa altura); a release leva o instalador NSIS e `SHA256SUMS` (sem `.sig` nem `latest.json`, que vêm com um atualizador numa versão futura; ajusta a secção 4.9); a prova real vai para o **Google Drive** (`XkyHold-wd real test`, para o lixo depois); o utilizador Windows padrão para o restauro limpo é criado pelo proprietário.

Melhorias pendentes vistas na Fase 7 (21/11/2026), sem data:
- **Restaurar, lista longa:** o botão "Restaurar N" só aparece no fim da lista (com a pasta de utilizador inteira, são dezenas de linhas). Proposta: barra de ações fixa no fundo do cartão, com "Voltar", o destino em resumo e "Restaurar N", sempre visível. Sugestão do proprietário; o lx tem o mesmo ecrã.
- **Junctions de sistema do perfil** ("Application Data", "Cookies", "My Documents"…): numa restauração para pasta nova noutro utilizador são recriadas a apontar para o perfil original (inofensivo: sem acesso e nunca seguidas). Ver se convém deixá-las de fora nesse caso.

## 8. Questões em aberto

Nenhuma de momento.

**Resolvida a 21/11/2026 — ligações relativas de Linux restauradas no Windows:** o proprietário decidiu trocar `/` por `\\` nos alvos relativos (decisão D5 na adenda `FORMAT-v2-windows.md` 5; o alvo guardado não muda). Engine com a alteração e teste (55 testes no Windows); visto na VM: a cópia do lx restaurada outra vez e `link-notas` passa a apontar para `projeto\\notas.md` e abre o ficheiro.

**Resolvida a 21/11/2026 — registar "só algumas pastas" na cópia:** o proprietário decidiu guardar a escolha. Adenda `xkyhold/docs/FORMAT-v2-chosen.md` (campo opcional `chosen` em cada raiz; rascunho para revisão); engine `f3781f7` escreve e lê o campo (52 testes em Linux, 53 no Windows); o lx 1.0.2 lista, restaura e verifica uma cópia com o campo. No wd, ligar a uma cópia traz a escolha preenchida em "O que guardar" (visto na VM: cópia com a escolha, "Remover", voltar a ligar, escolha de volta, cópia seguinte só dessa pasta). O "Concluir"/"Continuar" do assistente fica desligado enquanto "só algumas pastas" não tiver nenhuma pasta (antes contava como a pasta inteira). O lx pode receber o mesmo mais tarde.

**Também a 21/11/2026 — abrir uma cópia do Drive que já não existe:** antes, o motor criava uma pasta vazia com o nome dela (aconteceu com `XkyHold-wd test 3`, que estava no lixo). Agora o engine `f393f86` tem `DriveFolder::open_existing`, que nunca cria nada, e listar as cópias também não cria a pasta `XkyHold`; o wd só cria a pasta ao criar uma cópia nova. A janela diz "Esta cópia de segurança já não está no teu Google Drive. Se a puseste no lixo, recupera-a de lá e tenta outra vez." Visto na VM com uma cópia inexistente: a mensagem aparece e o Drive fica sem pasta nova. Engine: 53 testes em Linux, 54 no Windows.
