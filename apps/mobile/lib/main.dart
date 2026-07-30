import 'package:flutter/material.dart';
import 'package:flutter/services.dart';

import 'l10n/strings.dart';
import 'screens/home_screen.dart';
import 'screens/privacy_onboarding.dart';
import 'services/prefs.dart';

Future<void> main() async {
  WidgetsFlutterBinding.ensureInitialized();
  // FLAG_SECURE default ON (KD-23) — native channel later; document intent here.
  await SystemChrome.setPreferredOrientations([
    DeviceOrientation.portraitUp,
  ]);
  final prefs = await AppPrefs.load();
  runApp(ReceiptRadarApp(prefs: prefs));
}

class ReceiptRadarApp extends StatefulWidget {
  const ReceiptRadarApp({super.key, required this.prefs});

  final AppPrefs prefs;

  @override
  State<ReceiptRadarApp> createState() => _ReceiptRadarAppState();
}

class _ReceiptRadarAppState extends State<ReceiptRadarApp> {
  late AppPrefs prefs;

  @override
  void initState() {
    super.initState();
    prefs = widget.prefs;
  }

  void _finishOnboarding() {
    setState(() {
      prefs = prefs.copyWith(onboardingDone: true);
    });
    prefs.persist();
  }

  @override
  Widget build(BuildContext context) {
    final s = Strings.of(prefs.locale);
    return MaterialApp(
      title: s.appName,
      debugShowCheckedModeBanner: false,
      theme: ThemeData(
        colorScheme: ColorScheme.fromSeed(
          seedColor: const Color(0xFF0B6E4F),
          brightness: Brightness.light,
        ),
        useMaterial3: true,
      ),
      darkTheme: ThemeData(
        colorScheme: ColorScheme.fromSeed(
          seedColor: const Color(0xFF0B6E4F),
          brightness: Brightness.dark,
        ),
        useMaterial3: true,
      ),
      home: prefs.onboardingDone
          ? HomeScreen(prefs: prefs)
          : PrivacyOnboarding(strings: s, onDone: _finishOnboarding),
    );
  }
}
