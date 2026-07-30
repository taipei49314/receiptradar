/// Lightweight prefs without shared_preferences dep (file later).
/// v0.1 shell keeps onboarding flag in-memory + optional local file via path_provider later.
class AppPrefs {
  const AppPrefs({
    this.onboardingDone = false,
    this.locale = 'zh-TW',
    this.flagSecure = true,
  });

  final bool onboardingDone;
  final String locale;
  final bool flagSecure;

  AppPrefs copyWith({
    bool? onboardingDone,
    String? locale,
    bool? flagSecure,
  }) {
    return AppPrefs(
      onboardingDone: onboardingDone ?? this.onboardingDone,
      locale: locale ?? this.locale,
      flagSecure: flagSecure ?? this.flagSecure,
    );
  }

  static Future<AppPrefs> load() async {
    // Persistence via platform channels / shared_preferences in A21.
    return const AppPrefs();
  }

  Future<void> persist() async {
    // no-op until A21 settings store
  }
}
