import 'package:flutter/material.dart';

import '../l10n/strings.dart';

class PrivacyOnboarding extends StatelessWidget {
  const PrivacyOnboarding({
    super.key,
    required this.strings,
    required this.onDone,
  });

  final Strings strings;
  final VoidCallback onDone;

  @override
  Widget build(BuildContext context) {
    final cs = Theme.of(context).colorScheme;
    return Scaffold(
      body: SafeArea(
        child: Padding(
          padding: const EdgeInsets.all(24),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              const Spacer(),
              Icon(Icons.receipt_long, size: 72, color: cs.primary),
              const SizedBox(height: 16),
              Text(
                strings.appName,
                textAlign: TextAlign.center,
                style: Theme.of(context).textTheme.headlineMedium?.copyWith(
                      fontWeight: FontWeight.bold,
                    ),
              ),
              const SizedBox(height: 8),
              Text(
                strings.tagline,
                textAlign: TextAlign.center,
                style: Theme.of(context).textTheme.titleMedium,
              ),
              const SizedBox(height: 32),
              Text(
                strings.onboardingTitle,
                style: Theme.of(context).textTheme.titleLarge,
              ),
              const SizedBox(height: 12),
              Text(strings.onboardingBody),
              const SizedBox(height: 12),
              Text(
                strings.privacyNote,
                style: Theme.of(context).textTheme.bodySmall?.copyWith(
                      color: cs.outline,
                    ),
              ),
              const Spacer(),
              FilledButton(
                onPressed: onDone,
                child: Text(strings.continueLabel),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
