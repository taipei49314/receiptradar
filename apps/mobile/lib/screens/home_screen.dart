import 'package:flutter/material.dart';

import '../l10n/strings.dart';
import '../services/prefs.dart';
import 'capture_placeholder.dart';

class HomeScreen extends StatelessWidget {
  const HomeScreen({super.key, required this.prefs});

  final AppPrefs prefs;

  @override
  Widget build(BuildContext context) {
    final s = Strings.of(prefs.locale);
    return Scaffold(
      appBar: AppBar(
        title: Text(s.homeTitle),
        actions: [
          IconButton(
            tooltip: prefs.locale == 'zh-TW' ? 'EN' : '中文',
            onPressed: () {
              // Locale toggle needs Stateful parent — show snack for shell.
              ScaffoldMessenger.of(context).showSnackBar(
                SnackBar(content: Text(s.ffiPending)),
              );
            },
            icon: const Icon(Icons.language),
          ),
        ],
      ),
      body: Center(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: [
              Text(s.emptyLedger, textAlign: TextAlign.center),
              const SizedBox(height: 12),
              Text(
                s.privacyNote,
                style: Theme.of(context).textTheme.bodySmall,
                textAlign: TextAlign.center,
              ),
              const SizedBox(height: 8),
              Text(
                s.ffiPending,
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      color: Theme.of(context).colorScheme.outline,
                    ),
                textAlign: TextAlign.center,
              ),
            ],
          ),
        ),
      ),
      floatingActionButton: FloatingActionButton.extended(
        onPressed: () {
          Navigator.of(context).push(
            MaterialPageRoute<void>(
              builder: (_) => CapturePlaceholder(strings: s),
            ),
          );
        },
        icon: const Icon(Icons.photo_camera),
        label: Text(s.captureCta),
      ),
    );
  }
}
