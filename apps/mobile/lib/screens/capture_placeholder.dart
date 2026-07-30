import 'package:flutter/material.dart';

import '../l10n/strings.dart';

/// Camera + review sheet arrive with A20; this is navigation + UX scaffold.
class CapturePlaceholder extends StatelessWidget {
  const CapturePlaceholder({super.key, required this.strings});

  final Strings strings;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: Text(strings.captureCta)),
      body: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            AspectRatio(
              aspectRatio: 3 / 4,
              child: DecoratedBox(
                decoration: BoxDecoration(
                  color: Theme.of(context).colorScheme.surfaceContainerHighest,
                  borderRadius: BorderRadius.circular(12),
                  border: Border.all(
                    color: Theme.of(context).colorScheme.outlineVariant,
                  ),
                ),
                child: const Center(
                  child: Icon(Icons.camera_alt_outlined, size: 64),
                ),
              ),
            ),
            const SizedBox(height: 16),
            Text(strings.ffiPending),
            const Spacer(),
            FilledButton.tonal(
              onPressed: () => Navigator.of(context).pop(),
              child: const Text('OK'),
            ),
          ],
        ),
      ),
    );
  }
}
