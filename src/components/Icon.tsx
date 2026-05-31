import {
  Sun, Moon, Settings, ArrowLeft, Save, Video, Mic, ZoomIn, Sparkles,
  Keyboard, FolderOpen, Search, X, Info, CheckCircle, AlertCircle,
  Monitor, Globe, Code, Terminal, MessageCircle, Music, FileText,
  ScreenShare, Crosshair, RotateCcw, Play, Trash2, PauseCircle,
  StopCircle, Radio, Layout, Palette,
} from "lucide-react";

const iconMap: Record<string, React.ComponentType<{ size?: number; className?: string; style?: React.CSSProperties }>> = {
  light_mode: Sun,
  dark_mode: Moon,
  settings: Settings,
  arrow_back: ArrowLeft,
  save: Save,
  videocam: Video,
  mic: Mic,
  zoom_in: ZoomIn,
  auto_fix_high: Sparkles,
  keyboard: Keyboard,
  folder_open: FolderOpen,
  search: Search,
  close: X,
  info: Info,
  check_circle: CheckCircle,
  error: AlertCircle,
  screen_record: ScreenShare,
  capture: Crosshair,
  recenter: RotateCcw,
  desktop_windows: Monitor,
  public: Globe,
  code: Code,
  terminal: Terminal,
  chat: MessageCircle,
  music_note: Music,
  edit_note: FileText,
  web_asset: Layout,
  grid_view: Layout,
  play_arrow: Play,
  delete: Trash2,
  pause_circle: PauseCircle,
  stop_circle: StopCircle,
  radio_button_checked: Radio,
  filter_tilt_shift: Crosshair,
  palette: Palette,
};

interface IconProps {
  name: string;
  size?: number;
  className?: string;
  style?: React.CSSProperties;
}

export function Icon({ name, size = 18, className, style }: IconProps) {
  const IconComponent = iconMap[name];
  if (!IconComponent) {
    return <span style={{ fontSize: size, ...style }} className={className}>?</span>;
  }
  return <IconComponent size={size} className={className} style={style} />;
}
