import 'package:flutter/material.dart';

import '../l10n/strings.dart';
import '../services/rradar_api.dart';

/// Capture → process → confirm → attach closed-loop (mock until FRB/camera).
///
/// Mirrors Rust FFI `process_confirm_path_json` / `process_confirm_bytes_json`.
class CapturePlaceholder extends StatefulWidget {
  const CapturePlaceholder({
    super.key,
    required this.strings,
    this.api,
  });

  final Strings strings;
  final RradarApi? api;

  @override
  State<CapturePlaceholder> createState() => _CapturePlaceholderState();
}

class _CapturePlaceholderState extends State<CapturePlaceholder> {
  late final RradarApi _api = widget.api ?? MockRradarApi();
  bool _busy = false;
  String? _result;
  String? _error;

  Future<void> _runMockCapture() async {
    setState(() {
      _busy = true;
      _result = null;
      _error = null;
    });
    try {
      const mockPath = 'mock://camera/last_frame.jpg';
      final db = await _api.defaultLedgerPath();
      await _api.ensureLedger(db);
      final oneshot = await _api.processConfirmPath(
        dbPath: db,
        path: mockPath,
        confirm: true,
        attach: true,
        tags: 'capture,mobile',
        currency: 'TWD',
        engine: 'mock',
      );
      final n = await _api.count(db);
      final list = await _api.listJson(db, limit: 3);
      setState(() {
        _result =
            'Capture one-shot (mock ↔ FFI process_confirm_*)\n'
            '• process+confirm+attach ok\n'
            '• response: ${oneshot.length > 180 ? '${oneshot.substring(0, 180)}…' : oneshot}\n'
            '• ledger rows: $n\n'
            '• recent: $list\n'
            'Local-first · no cloud · schema v3 attachments.';
      });
    } catch (e) {
      setState(() => _error = e.toString());
    } finally {
      if (mounted) setState(() => _busy = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: Text(widget.strings.captureCta)),
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
                child: Center(
                  child: _busy
                      ? const CircularProgressIndicator()
                      : const Icon(Icons.camera_alt_outlined, size: 64),
                ),
              ),
            ),
            const SizedBox(height: 16),
            Text(
              widget.strings.ffiPending,
              style: Theme.of(context).textTheme.bodySmall,
            ),
            if (_result != null) ...[
              const SizedBox(height: 12),
              Text(
                _result!,
                style: Theme.of(context)
                    .textTheme
                    .bodyMedium
                    ?.copyWith(fontFamily: 'monospace'),
              ),
            ],
            if (_error != null) ...[
              const SizedBox(height: 12),
              Text(
                _error!,
                style: TextStyle(color: Theme.of(context).colorScheme.error),
              ),
            ],
            const Spacer(),
            FilledButton(
              onPressed: _busy ? null : _runMockCapture,
              child: Text(_busy ? '…' : 'Mock snap → process+confirm+attach'),
            ),
            const SizedBox(height: 8),
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
