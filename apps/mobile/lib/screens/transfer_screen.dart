import 'package:flutter/material.dart';

import '../l10n/strings.dart';
import '../services/rradar_api.dart';

/// Local multi-device transfer: CSV/JSON import + backup merge (mock / FFI).
///
/// No cloud relay — user picks a file (or mock demo) and merges into the
/// on-device ledger only.
class TransferScreen extends StatefulWidget {
  const TransferScreen({super.key, required this.strings});

  final Strings strings;

  @override
  State<TransferScreen> createState() => _TransferScreenState();
}

class _TransferScreenState extends State<TransferScreen> {
  String _log = '';
  int _count = 0;
  bool _busy = false;

  @override
  void initState() {
    super.initState();
    _refreshCount();
  }

  Future<void> _refreshCount() async {
    final path = await rradarApi.defaultLedgerPath();
    await rradarApi.ensureLedger(path);
    final n = await rradarApi.count(path);
    if (!mounted) return;
    setState(() => _count = n);
  }

  Future<void> _run(String label, Future<String> Function(String db) op) async {
    setState(() {
      _busy = true;
      _log = '… $label';
    });
    try {
      final db = await rradarApi.defaultLedgerPath();
      await rradarApi.ensureLedger(db);
      final out = await op(db);
      await _refreshCount();
      if (!mounted) return;
      setState(() {
        _log = '$label\n$out\nledger txs=$_count';
        _busy = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _log = 'error: $e';
        _busy = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final s = widget.strings;
    return Scaffold(
      appBar: AppBar(title: Text(s.transferTitle)),
      body: ListView(
        padding: const EdgeInsets.all(16),
        children: [
          Text(
            s.transferBlurb,
            style: Theme.of(context).textTheme.bodyMedium,
          ),
          const SizedBox(height: 8),
          Text(
            'ledger txs: $_count',
            style: Theme.of(context).textTheme.titleSmall?.copyWith(
                  fontFamily: 'monospace',
                ),
          ),
          const SizedBox(height: 16),
          FilledButton.icon(
            onPressed: _busy
                ? null
                : () => _run(
                      'import csv',
                      (db) => rradarApi.importCsvJson(
                        db,
                        'id,merchant,amount_minor,currency\n,CSV,1200,TWD',
                      ),
                    ),
            icon: const Icon(Icons.table_rows_outlined),
            label: Text(s.importCsv),
          ),
          const SizedBox(height: 8),
          OutlinedButton.icon(
            onPressed: _busy
                ? null
                : () => _run(
                      'import json',
                      (db) => rradarApi.importJsonJson(db, '[]'),
                    ),
            icon: const Icon(Icons.data_object),
            label: Text(s.importJson),
          ),
          const SizedBox(height: 8),
          OutlinedButton.icon(
            onPressed: _busy
                ? null
                : () => _run(
                      'backup merge',
                      rradarApi.backupMergeDemo,
                    ),
            icon: const Icon(Icons.sync_alt),
            label: Text(s.backupMerge),
          ),
          const SizedBox(height: 8),
          OutlinedButton.icon(
            onPressed: _busy
                ? null
                : () => _run(
                      'ocr lines',
                      (_) => rradarApi.ocrLinesJson(
                        pathOrLabel: 'mock://camera/frame.jpg',
                      ),
                    ),
            icon: const Icon(Icons.text_fields),
            label: Text(s.ocrRaw),
          ),
          const SizedBox(height: 24),
          Text(
            _log.isEmpty ? s.transferHint : _log,
            style: Theme.of(context).textTheme.bodySmall?.copyWith(
                  fontFamily: 'monospace',
                ),
          ),
        ],
      ),
    );
  }
}
