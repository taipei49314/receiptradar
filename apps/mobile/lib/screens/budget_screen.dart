import 'package:flutter/material.dart';

import '../l10n/strings.dart';
import '../services/rradar_api.dart';

/// Local soft budgets (mock until FRB). Never mixes currencies; no cloud.
class BudgetScreen extends StatefulWidget {
  const BudgetScreen({super.key, required this.strings});

  final Strings strings;

  @override
  State<BudgetScreen> createState() => _BudgetScreenState();
}

class _BudgetScreenState extends State<BudgetScreen> {
  String _raw = '[]';
  String? _error;
  bool _loading = true;

  @override
  void initState() {
    super.initState();
    _load();
  }

  Future<void> _load() async {
    setState(() {
      _loading = true;
      _error = null;
    });
    try {
      final path = await rradarApi.defaultLedgerPath();
      await rradarApi.ensureLedger(path);
      // Seed a capture so status has spend in mock.
      if (await rradarApi.count(path) == 0) {
        await rradarApi.processConfirmPath(
          dbPath: path,
          path: 'mock://budget/seed.jpg',
          tags: 'budget,demo',
        );
      }
      final st = await rradarApi.budgetStatusJson(path);
      if (!mounted) return;
      setState(() {
        _raw = st;
        _loading = false;
      });
    } catch (e) {
      if (!mounted) return;
      setState(() {
        _error = '$e';
        _loading = false;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Budgets'),
        actions: [
          IconButton(
            onPressed: _load,
            icon: const Icon(Icons.refresh),
          ),
        ],
      ),
      body: Padding(
        padding: const EdgeInsets.all(16),
        child: _loading
            ? const Center(child: CircularProgressIndicator())
            : _error != null
                ? Center(child: Text(_error!))
                : ListView(
                    children: [
                      Text(
                        'Local soft monthly limits · one currency per line · no cloud',
                        style: Theme.of(context).textTheme.bodySmall,
                      ),
                      const SizedBox(height: 12),
                      SelectableText(
                        _raw,
                        style: Theme.of(context).textTheme.bodySmall?.copyWith(
                              fontFamily: 'monospace',
                            ),
                      ),
                      const SizedBox(height: 16),
                      Text(
                        widget.strings.ffiPending,
                        style: Theme.of(context).textTheme.bodySmall,
                      ),
                    ],
                  ),
      ),
    );
  }
}
