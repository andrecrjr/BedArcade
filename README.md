# BedArcade

BedArcade transforma a nova aba do Chrome em um arcade de navegador: organize atalhos para jogos, ports e recompilações que rodam na web e navegue por eles com controle, teclado ou mouse. Cada jogo define seu próprio suporte a controle; a extensão não injeta compatibilidade dentro dele.

## Requisitos

- Node.js 24.10+
- Chrome
- Gamepad USB/Bluetooth compatível com Gamepad API

## Rodar

```bash
npm install
npm run dev
```

O WXT abre o Chrome de desenvolvimento quando configurado; alternativamente, abra `chrome://extensions`, ative **Modo do desenvolvedor** e carregue a pasta `.output/chrome-mv3` (gerada pelo WXT).

Para build de produção:

```bash
npm run build
npm run zip
```

## Integração contínua e releases

Pull requests para `main` executam formatação, lint, verificação de tipos e empacotamento da extensão. Quando uma alteração chega a `main`, o semantic-release analisa as mensagens dos commits e, se houver uma mudança lançável, cria a tag e a GitHub Release com notas e o ZIP versionado da extensão.

Use [Conventional Commits](https://www.conventionalcommits.org/) para que o semantic-release calcule a versão: `feat:` gera uma versão minor, `fix:` uma patch e uma mudança incompatível (`!` ou `BREAKING CHANGE:` no corpo) uma major. Por exemplo, `feat: add favorites` ou `fix: keep gamepad focus visible`. Commits como `docs:` e `chore:` não geram release por padrão. Em pull requests com squash merge, use esse padrão no título para que ele também apareça na mensagem do commit em `main`.

O job de release usa o `GITHUB_TOKEN` fornecido pelo GitHub Actions com permissão `contents: write`. O semantic-release atualiza a versão apenas durante o build para que o manifesto e o ZIP coincidam; `npmPublish: false` impede publicação no npm. Não são necessárias credenciais da Chrome Web Store nem secrets adicionais. Como ainda não há tags de release, a primeira versão criada seguirá o padrão inicial do semantic-release (`v1.0.0`).

## Recursos visuais

- `public/bedarcade-icon.svg` é a origem vetorial do ícone da marca.
- `public/bedarcade-logo.png` é a versão raster do ícone da marca.
- `public/icons/` contém os ícones de 16, 32, 48 e 128 px usados pelo Chrome e como favicon das páginas da extensão.
- `public/bedarcade-poster.svg` é a origem vetorial do banner promocional; `public/bedarcade-poster.png` é a versão de 1400 × 560 px para o site ou a página da extensão.
- `public/gamepad-icons/` contém prompts em SVG de Xbox e PlayStation do pacote CC0 Kenney Input Prompts; veja `ASSET_CREDITS.md`.

## Testar

Os testes E2E usam Playwright com a extensão WXT em modo `e2e`. Na primeira execução, instale o Chromium do Playwright e rode a suíte:

```bash
npx playwright install chromium
npm run test:e2e
```

O comando compila a extensão de teste em `.output/chrome-mv3-e2e` e verifica o launcher, a navegação em páginas locais simuladas e a persistência dos atalhos. Esse modo injeta quadros de controle previsíveis; a build normal continua lendo o Gamepad API do navegador.

Para validar um controle físico manualmente:

1. Instale a build de `.output/chrome-mv3` e abra uma nova aba.
2. Conecte o controle e pressione um botão para que o Chrome o reconheça.
3. Use D-pad/analógico esquerdo para navegar pelos atalhos ou pelo teclado virtual.
4. A/X clica ou escolhe; Start envia formulários; Select pausa e retoma.
5. Com o teclado virtual aberto, B o fecha; fora dele, B volta. LB/RB trocam abas.
6. Os atalhos iniciais incluem AC-JR, YouTube, Google, Steam, CrazyGames e jogos de navegador do itch.io. Adicione, edite ou remova atalhos no launcher.

## Limitações conhecidas

- O override da nova aba não é usado em janelas anônimas do Chrome.
- Sem suporte a `chrome://`, Chrome Web Store e outras páginas protegidas.
- Não alcança elementos internos de shadow roots fechados, iframes ou canvas.
- `.click()` não cria eventos `isTrusted`, portanto alguns sites podem bloquear ações.
- O teclado flutuante aparece ao focar campos de texto e áreas de texto em páginas HTTP e HTTPS compatíveis e na nova aba.
- A seleção usa elementos visíveis no viewport; controles com telas extensas podem exigir rolagem.
- Mapeamento assume `mapping === 'standard'` (gamepads não padronizados podem divergir).
- Os atalhos ficam salvos no perfil local do Chrome e não são sincronizados entre dispositivos.
- Cada jogo define o próprio suporte ao controle; a extensão navega por sites e seus controles padrão, mas não converte entradas para jogos sem suporte.
- A suíte automatizada cobre Chromium e mapeamentos padrão; controles físicos ainda devem ser validados manualmente.
