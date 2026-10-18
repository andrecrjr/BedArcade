# Gamepad Navigator

Extensão Chrome Manifest V3 com WXT + TypeScript. A página de nova aba funciona como um launcher para navegar com controle e teclado.

## Requisitos

- Node.js 20+
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

## Recursos visuais

- `public/gamepad-navigator-icon.svg` é a origem vetorial da marca.
- `public/icons/` contém os ícones de 16, 32, 48 e 128 px usados pelo Chrome e como favicon das páginas da extensão.
- `public/gamepad-navigator-poster.png` é o banner promocional de 1400 × 560 px para o site ou a página da extensão.
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
