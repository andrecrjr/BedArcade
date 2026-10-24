# Gamepad Desktop

Use o controle para navegar pelo computador, mover o ponteiro e digitar sem tirar o foco do aplicativo que você está usando.

O app controla o ponteiro e envia comandos de teclado, além de oferecer um teclado visual próprio. Ele continua funcionando quando o painel é recolhido para a bandeja do sistema.

Notificações do sistema avisam sobre pausa e retomada, conexão do controle e falhas mesmo com o painel recolhido. No Windows, a primeira execução cria o atalho por usuário “Gamepad Desktop” no menu Iniciar para identificar os avisos.

> O desktop está em desenvolvimento. Cada aplicativo interpreta as teclas recebidas à sua maneira, e ainda falta validar o uso com controles físicos em Windows e Linux.

![Prévia do teclado visual](keyboard-preview.png)

## Começar

No Windows, baixe e extraia o [pacote x64](dist/GamepadDesktop-Windows-x64.zip), conecte o controle e abra `gamepad-desktop.exe`. O painel mostra quando o controle foi reconhecido e quais comandos estão disponíveis.

No Linux, baixe e extraia o [pacote x64](dist/GamepadDesktop-Linux-x64.tar.gz). As instruções para executar o app e as permissões necessárias estão no [guia do sistema](docs/system-overview.md#como-iniciar-no-linux).

## Comandos principais

| Controle                             | Ação                                                   |
| ------------------------------------ | ------------------------------------------------------ |
| Analógico esquerdo                    | Mover o ponteiro                                        |
| Analógico direito, cima/baixo         | Rolar a página                                          |
| Direcional                            | Enviar setas; segurar para repetir                      |
| Botão inferior — A / ✕ / B no Switch  | Clicar; no teclado visual, digitar a tecla selecionada  |
| Botão direito — B / ○ / A no Switch   | Clique direito                                           |
| Botão esquerdo — X / □ / Y no Switch  | Enter                                                    |
| Botão superior — Y / △ / X no Switch  | Escape                                                   |
| L3                                    | Abrir o lançador do sistema                             |
| LB / L1                               | Tab                                                      |
| RB / R1                               | Alt+Tab                                                  |
| R3 — pressionar o analógico direito   | Abrir ou fechar o teclado visual                        |
| Start + Select / Menu + View          | Pausar ou retomar; solte os botões e centralize para retomar |

Os nomes e comandos completos aparecem no painel, incluindo os equivalentes de Xbox, PlayStation e Switch. A seção de ajustes permite escolher o controle, ajustar velocidade e personalizar o teclado.

## O que saber antes de usar

- As setas do direcional são enviadas ao aplicativo ativo; o suporte depende do próprio aplicativo.
- Nos painéis nativos do Windows, o app verifica a janela ativa, o controle focado e sua janela raiz para evitar duplicar a navegação do direcional. Isso inclui as configurações rápidas hospedadas por `System32\ShellHost.exe` (`ControlCenterWindow`) no Windows 11 recente.
- No Windows, pausar com Start + Select oculta os cursores padrão enquanto há um controle conectado. Retomar, desconectar o controle ou encerrar normalmente pela bandeja restaura o esquema de cursores. Cursores personalizados de aplicativos podem continuar visíveis. Se o processo for encerrado à força durante a pausa, reaplique o esquema em Configurações do mouse → Ponteiros para restaurá-los.
- O teclado visual usa o layout QWERTY em inglês. No Linux, símbolos e acentos podem variar conforme o layout ativo.
- Em algumas sessões Wayland, incluindo o GNOME Wayland, o teclado visual não está disponível.
- Os pacotes Windows e Linux existem, mas o uso com controle físico ainda precisa de validação em Windows 11, Linux/X11 e Wayland.

## Documentação

- [Como o sistema funciona](docs/system-overview.md): controles, fluxo do controle, diferenças entre sistemas, limites e execução.
- [Pesquisa do teclado visual](docs/keyboard-research.md): decisões de layout, foco e posicionamento.
- [Roadmap](docs/roadmap.md): trabalho concluído e validações pendentes.
