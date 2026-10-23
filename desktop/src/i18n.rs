#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Locale {
    #[default]
    #[serde(rename = "en")]
    English,
    #[serde(rename = "es")]
    Spanish,
    #[serde(rename = "pt-BR")]
    PortugueseBrazil,
}

impl Locale {
    pub const ALL: [Self; 3] = [Self::English, Self::Spanish, Self::PortugueseBrazil];

    pub const fn name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Spanish => "Español",
            Self::PortugueseBrazil => "Português (Brasil)",
        }
    }

    pub const fn text(self, text: Text) -> &'static str {
        text.get(self)
    }

    pub fn translate(self, message: &str) -> String {
        let translated = match message {
            "Direcional" => Some(Text::Dpad),
            "R3 · pressionar analógico direito" => Some(Text::RightStickPress),
            "L3 · pressionar analógico esquerdo" => Some(Text::LeftStickPress),
            "Analógico esquerdo" => Some(Text::LeftStick),
            "Analógico direito ↕" => Some(Text::RightStickVertical),
            "Inferior · A / ✕ / B no Switch" => Some(Text::SouthButton),
            "Direito · B / ○ / A no Switch" => Some(Text::EastButton),
            "Esquerdo · X / □ / Y no Switch" => Some(Text::WestButton),
            "Superior · Y / △ / X no Switch" => Some(Text::NorthButton),
            "A / botão inferior" => Some(Text::SouthButtonShort),
            "B / botão direito" => Some(Text::EastButtonShort),
            "X / botão esquerdo" => Some(Text::WestButtonShort),
            "Y / botão superior" => Some(Text::NorthButtonShort),
            "Abrir / fechar teclado visual próprio" => Some(Text::ToggleKeyboard),
            "Lançador do sistema · Super / Windows" => Some(Text::SystemLauncher),
            "Mover ponteiro" => Some(Text::MovePointer),
            "Rolar página" => Some(Text::ScrollPage),
            "Clique esquerdo · segurar para arrastar" => Some(Text::LeftClickDrag),
            "Clique direito" => Some(Text::RightClick),
            "Enter · abrir item selecionado" => Some(Text::EnterSelected),
            "Escape · fechar menu / cancelar" => Some(Text::EscapeCancel),
            "Setas · segurar para repetir" => Some(Text::ArrowRepeat),
            "Tab · próximo campo" => Some(Text::NextField),
            "Alt+Tab · alternar janela" => Some(Text::SwitchWindow),
            "Start + Select / Menu + View" => Some(Text::PauseCombo),
            "Pausar / retomar · solte e centralize para retomar" => Some(Text::PauseResume),
            "Mover a seleção pelo teclado" => Some(Text::MoveKeyboardSelection),
            "Digitar a tecla selecionada" => Some(Text::TypeSelectedKey),
            "Apagar um caractere" => Some(Text::DeleteCharacter),
            "Inserir Enter" => Some(Text::InsertEnter),
            "Inserir Esc" => Some(Text::InsertEscape),
            "Inserir espaço" => Some(Text::InsertSpace),
            "Inserir Tab" => Some(Text::InsertTab),
            "Ativar ou desativar Shift" => Some(Text::ToggleShift),
            "Fechar o teclado visual" => Some(Text::CloseKeyboard),
            "Ajustes do teclado · direcional escolhe e altera" => Some(Text::KeyboardSettingsHelp),
            "Perfil aplicado e salvo para as próximas sessões." => Some(Text::ProfileSaved),
            "Este compositor não permite exibir o teclado próprio sem ativá-lo." => {
                Some(Text::KeyboardOverlayUnavailable)
            }
            "Teclado visual próprio · direcional move · A digita · B apaga · R3 fecha." => {
                Some(Text::KeyboardOpened)
            }
            "Teclado visual fechado." => Some(Text::KeyboardClosed),
            "Cole uma única linha de perfil SDL: GUID,nome,botões/eixos." => {
                Some(Text::PasteOneSdlMapping)
            }
            _ => None,
        };
        if let Some(text) = translated {
            return self.text(text).to_owned();
        }

        for (prefix, text) in [
            ("Erro ao carregar perfil SDL: ", Text::SdlLoadProfilePrefix),
            ("Erro ao ler os perfis: ", Text::SdlReadProfilesPrefix),
            (
                "Perfil aplicado nesta sessão, mas não foi salvo: ",
                Text::ProfileSaveFailedPrefix,
            ),
            (
                "Não foi possível salvar ajustes: ",
                Text::SettingsSaveFailedPrefix,
            ),
            (
                "Falha ao conectar entrada do Windows: ",
                Text::WindowsConnectFailedPrefix,
            ),
            (
                "Falha enviando entrada ao Windows: ",
                Text::WindowsSendFailedPrefix,
            ),
            (
                "Erro enviando entrada ao desktop: ",
                Text::DesktopSendFailedPrefix,
            ),
            ("SDL3: ", Text::SdlErrorPrefix),
        ] {
            if let Some(detail) = message.strip_prefix(prefix) {
                return format!("{}{detail}", self.text(text));
            }
        }
        message.to_owned()
    }

    pub fn detect() -> Self {
        #[cfg(target_os = "windows")]
        if let Some(identifier) = windows_locale() {
            return Self::from_identifier(&identifier);
        }

        ["LC_ALL", "LC_MESSAGES", "LANG"]
            .into_iter()
            .filter_map(|name| std::env::var(name).ok())
            .find(|value| !value.is_empty())
            .map_or(Self::English, |identifier| {
                Self::from_identifier(&identifier)
            })
    }

    fn from_identifier(identifier: &str) -> Self {
        let language = identifier
            .split(['.', '@'])
            .next()
            .unwrap_or(identifier)
            .replace('_', "-")
            .to_ascii_lowercase();
        if language.starts_with("pt") {
            Self::PortugueseBrazil
        } else if language.starts_with("es") {
            Self::Spanish
        } else {
            Self::English
        }
    }
}

#[cfg(target_os = "windows")]
fn windows_locale() -> Option<String> {
    let mut name = [0_u16; 85];
    let length = unsafe { windows::Win32::Globalization::GetUserDefaultLocaleName(&mut name) };
    (length > 0).then(|| String::from_utf16_lossy(&name[..length as usize]))
}

macro_rules! catalog {
    ($($key:ident => ($en:literal, $es:literal, $pt:literal);)+) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum Text {
            $($key,)+
        }

        impl Text {
            const fn get(self, locale: Locale) -> &'static str {
                match self {
                    $(Self::$key => match locale {
                        Locale::English => $en,
                        Locale::Spanish => $es,
                        Locale::PortugueseBrazil => $pt,
                    },)+
                }
            }
        }
    };
}

catalog! {
    Donate => ("Donate", "Donar", "Apoiar");
    Projects => ("My Projects", "Mis proyectos", "Meus projetos");
    TrayOpen => ("Open panel", "Abrir panel", "Abrir painel");
    TrayHide => ("Hide panel", "Ocultar panel", "Recolher painel");
    DesktopControl => ("Desktop control", "Control del escritorio", "Controle do desktop");
    Connecting => ("Connecting to the system…", "Conectando al sistema…", "Conectando ao sistema…");
    Retry => ("Try again", "Intentar de nuevo", "Tentar novamente");
    ControllerPrefix => ("Controller: ", "Control: ", "Controle: ");
    KeyboardActive => ("On-screen keyboard · D-pad moves · A types · R3 closes", "Teclado en pantalla · la cruceta mueve · A escribe · R3 cierra", "Teclado visual próprio · direcional move · A digita · R3 fecha");
    MouseMode => ("Mouse mode · release buttons and center sticks when connecting or resuming", "Modo ratón · suelta los botones y centra los sticks al conectar o reanudar", "Modo mouse · solte os botões e centralize os analógicos ao conectar ou retomar");
    Paused => ("Paused · press Start + Select to resume", "En pausa · pulsa Start + Select para reanudar", "Pausado · pressione Start + Select para retomar");
    ConnectGamepad => ("Connect a gamepad. Detection is automatic.", "Conecta un mando. La detección es automática.", "Conecte um gamepad. A detecção é automática.");
    SelectController => ("Select a controller with a profile or import one below.", "Selecciona un mando con perfil o importa uno abajo.", "Selecione um controle com perfil ou importe um abaixo.");
    DeviceSupport => ("USB/Bluetooth · Xbox, PlayStation, Switch and SDL profiles for generic devices.", "USB/Bluetooth · Xbox, PlayStation, Switch y perfiles SDL para mandos genéricos.", "USB/Bluetooth · Xbox, PlayStation, Switch e perfis SDL para genéricos.");
    KeyboardVisual => ("ON-SCREEN KEYBOARD", "TECLADO EN PANTALLA", "TECLADO VISUAL");
    PausedLabel => ("PAUSED", "EN PAUSA", "PAUSADO");
    MouseLabel => ("MOUSE", "RATÓN", "MOUSE");
    DesktopCommands => ("Desktop controls", "Controles del escritorio", "Comandos do desktop");
    KeyboardCommands => ("On-screen keyboard controls", "Controles del teclado en pantalla", "Comandos do teclado visual");
    ButtonPositionNote => ("South, east, west, and north refer to the physical positions of the four face buttons. Escape cancels menus; it is not the browser Back command.", "Sur, este, oeste y norte indican la posición física de los cuatro botones. Escape cancela menús; no es el comando Atrás del navegador.", "Inferior, direito, esquerdo e superior indicam a posição física dos quatro botões. Escape cancela menus; não é o comando Voltar do navegador.");
    KeyboardFocusNote => ("The keyboard does not focus its own window; the text field and cursor stay in the previous app.", "El teclado no enfoca su propia ventana; el campo y el cursor permanecen en la aplicación anterior.", "O teclado não ativa a própria janela; o campo e o cursor de texto permanecem no aplicativo anterior.");
    Settings => ("Settings", "Configuración", "Configurações");
    StartWithOS => ("Run when I sign in (starts minimized)", "Iniciar al iniciar sesión (minimizado)", "Iniciar com o sistema (minimizado)");
    Language => ("Language", "Idioma", "Idioma");
    PreferencesSaveFailedPrefix => ("Could not save preferences: ", "No se pudo guardar la configuración: ", "Não foi possível salvar as configurações: ");
    ControlsAndAdjustments => ("Controller and adjustments", "Mando y ajustes", "Controle e ajustes");
    Automatic => ("Automatic", "Automático", "Automático");
    DisconnectedController => ("Disconnected controller", "Mando desconectado", "Controle desconectado");
    NeedsProfile => (" · profile required", " · necesita un perfil", " · precisa de perfil");
    ControlDesktop => ("Control desktop (Start + Select pauses/resumes)", "Controlar el escritorio (Start + Select pausa/reanuda)", "Controlar desktop (Start + Select pausa/retoma)");
    NotificationTitle => ("Gamepad Desktop", "Gamepad Desktop", "Gamepad Desktop");
    PauseCombo => ("Start + Select / Menu + View", "Start + Select / Menú + Vista", "Start + Select / Menu + View");
    ControlPausedNotice => ("Control paused. Press START + SELECT to resume.", "Control pausado. Pulsa START + SELECT para reanudar.", "Controle pausado. Pressione START + SELECT para retomar.");
    ControlResumedNotice => ("Control resumed. Release all buttons and center both sticks to continue.", "Control reanudado. Suelta los botones y centra ambos sticks para continuar.", "Controle retomado. Solte os botões e centralize os dois analógicos para continuar.");
    ControllerConnectedPrefix => ("Controller connected: ", "Control conectado: ", "Controle conectado: ");
    ControllerDisconnectedNotice => ("Controller disconnected. Reconnect it to resume control.", "Control desconectado. Reconéctalo para volver a controlar el escritorio.", "Controle desconectado. Reconecte-o para voltar a controlar o desktop.");
    NoUsableProfileNotice => ("No usable controller profile was found. Import an SDL mapping in the desktop panel.", "No se encontró un perfil de control compatible. Importa un mapeo SDL en el panel del escritorio.", "Nenhum perfil de controle compatível foi encontrado. Importe um mapeamento SDL no painel do desktop.");
    KeyboardUnavailableNotice => ("The on-screen keyboard could not be opened.", "No se pudo abrir el teclado en pantalla.", "Não foi possível abrir o teclado visual.");
    RuntimeStoppedPrefix => ("Desktop control stopped: ", "El control del escritorio se detuvo: ", "O controle do desktop parou: ");
    Speed => ("Speed", "Velocidad", "Velocidade");
    Deadzone => ("Deadzone", "Zona muerta", "Zona morta");
    CustomizeKeyboard => ("Customize QWERTY keyboard", "Personalizar teclado QWERTY", "Personalizar teclado QWERTY");
    WidthPx => ("Width (px)", "Ancho (px)", "Largura (px)");
    Top => ("Top", "Arriba", "Em cima");
    Bottom => ("Bottom", "Abajo", "Embaixo");
    Opacity => ("Opacity", "Opacidad", "Opacidade");
    RepeatDirection => ("Repeat D-pad while held", "Repetir la cruceta al mantenerla", "Repetir direcional ao segurar");
    InitialWait => ("Initial delay (ms)", "Espera inicial (ms)", "Espera inicial (ms)");
    RepeatInterval => ("Repeat interval (ms)", "Intervalo de repetición (ms)", "Intervalo (ms)");
    RestoreKeyboard => ("Restore keyboard defaults", "Restaurar ajustes del teclado", "Restaurar ajustes do teclado");
    KeyboardFollowsField => ("The keyboard follows the field when an app provides its position. Top/Bottom sets the fallback position when that information is unavailable.", "El teclado sigue el campo cuando la aplicación informa su posición. Arriba/Abajo define la posición alternativa cuando no está disponible.", "O teclado acompanha o campo quando o aplicativo fornece sua posição. Em cima/Embaixo define a alternativa quando essa informação não está disponível.");
    KeyboardSettingsHelp => ("With the keyboard open, Start opens these settings from the gamepad. Preferences are saved when you return.", "Con el teclado abierto, Start abre estos ajustes desde el mando. Las preferencias se guardan al volver.", "Com o teclado aberto, Start também abre estes ajustes pelo gamepad. As preferências são salvas ao voltar.");
    KeyboardBehaviorHelp => ("ANSI QWERTY · Shift applies to the next letter · Caps locks uppercase · Ctrl, Alt, and Super combine with the next key.", "QWERTY ANSI · Shift se aplica a la siguiente letra · Caps fija mayúsculas · Ctrl, Alt y Super se combinan con la siguiente tecla.", "QWERTY ANSI · Shift vale para a próxima letra · Caps fixa maiúsculas · Ctrl, Alt e Super combinam com a próxima tecla.");
    TrayUnavailablePrefix => ("Tray unavailable: ", "Bandeja no disponible: ", "Bandeja indisponível: ");
    ClosePanelKeepsRunning => ("Closing the panel keeps the controller active in the tray near the clock.", "Al cerrar el panel, el mando sigue activo en la bandeja junto al reloj.", "Fechar o painel mantém o controle ativo na bandeja, perto do relógio.");
    QuitApplication => ("Quit application", "Cerrar la aplicación", "Encerrar aplicativo");
    ControllerProfiles => ("Controller profiles", "Perfiles de mandos", "Perfis de controles");
    KnownProfiles => ("Known controllers use SDL profiles automatically. For an unknown model, import an SDL mapping line.", "Los mandos conocidos usan perfiles SDL automáticamente. Para un modelo desconocido, importa una línea de mapeo SDL.", "Controles conhecidos usam o perfil SDL automaticamente. Para um modelo desconhecido, importe uma linha de mapeamento SDL.");
    ApplySaveProfile => ("Apply and save profile", "Aplicar y guardar perfil", "Aplicar e salvar perfil");
    ProfileSaved => ("Profile applied and saved for future sessions.", "Perfil aplicado y guardado para próximas sesiones.", "Perfil aplicado e salvo para as próximas sessões.");
    KeyboardSettingsTitle => ("KEYBOARD SETTINGS", "AJUSTES DEL TECLADO", "AJUSTES DO TECLADO");
    KeyboardWindowTitle => ("Gamepad Desktop · Keyboard", "Gamepad Desktop · Teclado", "Gamepad Desktop · Teclado");
    KeyboardMoveAdjust => ("Up/down selects · left/right adjusts", "Arriba/abajo selecciona · izquierda/derecha ajusta", "Cima/baixo escolhe · lados altera");
    KeyboardWidth => ("Width", "Ancho", "Tamanho");
    KeyboardPosition => ("Position", "Posición", "Posição");
    KeyboardOpacity => ("Opacity", "Opacidad", "Opacidade");
    KeyboardRepeat => ("Repeat while held", "Repetir al mantener", "Repetir ao segurar");
    KeyboardRepeatDelay => ("Delay before repeat", "Espera antes de repetir", "Espera antes de repetir");
    KeyboardRepeatInterval => ("Repeat interval", "Intervalo de repetición", "Intervalo de repetição");
    Yes => ("yes", "sí", "sim");
    No => ("no", "no", "não");
    KeyboardCustomizeHelp => ("Start returns and saves · R3 closes", "Start vuelve y guarda · R3 cierra", "Start voltar e salvar · R3 fechar");
    KeyboardTypingHelp => ("A type · B delete · L3 space · R1 Shift · Start settings", "A escribir · B borrar · L3 espacio · R1 Shift · Start ajustes", "A digitar · B apagar · L3 espaço · R1 Shift · Start ajustes");
    KeyboardNavigationHelp => ("X Enter · Y Esc · L1 Tab · R3 close", "X Enter · Y Esc · L1 Tab · R3 cerrar", "X Enter · Y Esc · L1 Tab · R3 fechar");
    Space => ("Space", "Espacio", "Espaço");
    KeyboardOverlayUnavailable => ("This compositor cannot show the custom keyboard without activating it.", "Este compositor no puede mostrar el teclado propio sin activarlo.", "Este compositor não permite exibir o teclado próprio sem ativá-lo.");
    KeyboardOpened => ("On-screen keyboard · D-pad moves · A types · B deletes · R3 closes.", "Teclado en pantalla · la cruceta mueve · A escribe · B borra · R3 cierra.", "Teclado visual próprio · direcional move · A digita · B apaga · R3 fecha.");
    KeyboardClosed => ("On-screen keyboard closed.", "Teclado en pantalla cerrado.", "Teclado visual fechado.");
    PasteOneSdlMapping => ("Paste one SDL profile line: GUID,name,buttons/axes.", "Pega una línea de perfil SDL: GUID,nombre,botones/ejes.", "Cole uma única linha de perfil SDL: GUID,nome,botões/eixos.");
    SdlLoadProfilePrefix => ("Could not load SDL profile: ", "No se pudo cargar el perfil SDL: ", "Erro ao carregar perfil SDL: ");
    SdlReadProfilesPrefix => ("Could not read profiles: ", "No se pudieron leer los perfiles: ", "Erro ao ler os perfis: ");
    ProfileSaveFailedPrefix => ("Profile applied for this session, but could not be saved: ", "Perfil aplicado para esta sesión, pero no se pudo guardar: ", "Perfil aplicado nesta sessão, mas não foi salvo: ");
    SettingsSaveFailedPrefix => ("Could not save settings: ", "No se pudieron guardar los ajustes: ", "Não foi possível salvar ajustes: ");
    WindowsConnectFailedPrefix => ("Could not connect Windows input: ", "No se pudo conectar la entrada de Windows: ", "Falha ao conectar entrada do Windows: ");
    WindowsSendFailedPrefix => ("Could not send input to Windows: ", "No se pudo enviar la entrada a Windows: ", "Falha enviando entrada ao Windows: ");
    DesktopSendFailedPrefix => ("Could not send input to the desktop: ", "No se pudo enviar la entrada al escritorio: ", "Erro enviando entrada ao desktop: ");
    SdlErrorPrefix => ("SDL3: ", "SDL3: ", "SDL3: ");
    Dpad => ("D-pad", "Cruceta", "Direcional");
    RightStickPress => ("R3 · press right stick", "R3 · pulsar stick derecho", "R3 · pressionar analógico direito");
    LeftStickPress => ("L3 · press left stick", "L3 · pulsar stick izquierdo", "L3 · pressionar analógico esquerdo");
    LeftStick => ("Left stick", "Stick izquierdo", "Analógico esquerdo");
    RightStickVertical => ("Right stick ↕", "Stick derecho ↕", "Analógico direito ↕");
    SouthButton => ("South · A / ✕ / B on Switch", "Sur · A / ✕ / B en Switch", "Inferior · A / ✕ / B no Switch");
    EastButton => ("East · B / ○ / A on Switch", "Este · B / ○ / A en Switch", "Direito · B / ○ / A no Switch");
    WestButton => ("West · X / □ / Y on Switch", "Oeste · X / □ / Y en Switch", "Esquerdo · X / □ / Y no Switch");
    NorthButton => ("North · Y / △ / X on Switch", "Norte · Y / △ / X en Switch", "Superior · Y / △ / X no Switch");
    SouthButtonShort => ("A / south button", "A / botón sur", "A / botão inferior");
    EastButtonShort => ("B / east button", "B / botón este", "B / botão direito");
    WestButtonShort => ("X / west button", "X / botón oeste", "X / botão esquerdo");
    NorthButtonShort => ("Y / north button", "Y / botón norte", "Y / botão superior");
    ToggleKeyboard => ("Open / close custom on-screen keyboard", "Abrir / cerrar teclado en pantalla", "Abrir / fechar teclado visual próprio");
    SystemLauncher => ("System launcher · Super / Windows", "Lanzador del sistema · Super / Windows", "Lançador do sistema · Super / Windows");
    MovePointer => ("Move pointer", "Mover el puntero", "Mover ponteiro");
    ScrollPage => ("Scroll page", "Desplazar página", "Rolar página");
    LeftClickDrag => ("Left click · hold to drag", "Clic izquierdo · mantener para arrastrar", "Clique esquerdo · segurar para arrastar");
    RightClick => ("Right click", "Clic derecho", "Clique direito");
    EnterSelected => ("Enter · open selected item", "Enter · abrir elemento seleccionado", "Enter · abrir item selecionado");
    EscapeCancel => ("Escape · close menu / cancel", "Escape · cerrar menú / cancelar", "Escape · fechar menu / cancelar");
    ArrowRepeat => ("Arrow keys · hold to repeat", "Flechas · mantener para repetir", "Setas · segurar para repetir");
    NextField => ("Tab · next field", "Tab · campo siguiente", "Tab · próximo campo");
    SwitchWindow => ("Alt+Tab · switch window", "Alt+Tab · cambiar ventana", "Alt+Tab · alternar janela");
    PauseResume => ("Pause / resume · release and center to resume", "Pausar / reanudar · suelta y centra para reanudar", "Pausar / retomar · solte e centralize para retomar");
    MoveKeyboardSelection => ("Move selection across the keyboard", "Mover la selección por el teclado", "Mover a seleção pelo teclado");
    TypeSelectedKey => ("Type the selected key", "Escribir la tecla seleccionada", "Digitar a tecla selecionada");
    DeleteCharacter => ("Delete one character", "Borrar un carácter", "Apagar um caractere");
    InsertEnter => ("Insert Enter", "Insertar Enter", "Inserir Enter");
    InsertEscape => ("Insert Escape", "Insertar Esc", "Inserir Esc");
    InsertSpace => ("Insert space", "Insertar espacio", "Inserir espaço");
    InsertTab => ("Insert Tab", "Insertar Tab", "Inserir Tab");
    ToggleShift => ("Toggle Shift", "Activar o desactivar Shift", "Ativar ou desativar Shift");
    CloseKeyboard => ("Close on-screen keyboard", "Cerrar teclado en pantalla", "Fechar o teclado visual");
}
