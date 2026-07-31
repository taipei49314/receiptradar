/// Minimal zh-TW / en strings (ARB full i18n later).
class Strings {
  const Strings({
    required this.locale,
    required this.appName,
    required this.tagline,
    required this.onboardingTitle,
    required this.onboardingBody,
    required this.continueLabel,
    required this.homeTitle,
    required this.captureCta,
    required this.emptyLedger,
    required this.privacyNote,
    required this.ffiPending,
    required this.aboutTitle,
    required this.ledgerTitle,
    required this.refresh,
  });

  final String locale;
  final String appName;
  final String tagline;
  final String onboardingTitle;
  final String onboardingBody;
  final String continueLabel;
  final String homeTitle;
  final String captureCta;
  final String emptyLedger;
  final String privacyNote;
  final String ffiPending;
  final String aboutTitle;
  final String ledgerTitle;
  final String refresh;

  static Strings of(String locale) {
    if (locale.startsWith('en')) {
      return const Strings(
        locale: 'en',
        appName: 'ReceiptRadar',
        tagline: 'Snap. Parse. Own your spending.',
        onboardingTitle: 'No cloud. No account.',
        onboardingBody:
            'Receipt images and your ledger stay on this device by default. '
            'Any network feature is opt-in. You can export an encrypted backup anytime.',
        continueLabel: 'Continue',
        homeTitle: 'Home',
        captureCta: 'Capture receipt',
        emptyLedger: 'No transactions yet. Capture a receipt to start.',
        privacyNote: 'Local-first · FLAG_SECURE on by default',
        ffiPending:
            'Rust FFI contract ready (rradar-ffi). FRB codegen when Flutter SDK is available.',
        aboutTitle: 'About / capabilities',
        ledgerTitle: 'Ledger',
        refresh: 'Refresh',
      );
    }
    return const Strings(
      locale: 'zh-TW',
      appName: '發票雷達',
      tagline: '拍下發票，帳本留下——不上雲。',
      onboardingTitle: '不上雲、免帳號',
      onboardingBody: '收據影像與帳本預設只留在這台裝置。任何網路功能皆需你主動開啟。'
          '可隨時匯出加密備份。',
      continueLabel: '開始使用',
      homeTitle: '首頁',
      captureCta: '拍攝收據',
      emptyLedger: '尚無交易。拍一張收據開始記帳。',
      privacyNote: '本地優先 · 預設防截圖',
      ffiPending: 'Rust FFI 契約已就緒（rradar-ffi）。有 Flutter SDK 後再 generate FRB。',
      aboutTitle: '關於／能力',
      ledgerTitle: '帳本',
      refresh: '重新整理',
    );
  }
}
