/// Abstract ReceiptRadar core API for Flutter UI.
///
/// Mirrors `crates/rradar-ffi` free functions. Until flutter_rust_bridge
/// generates native bindings, [MockRradarApi] drives the shell UI.
///
/// Full function map: `docs/frb-contract.md`.
library;

/// Capability blob (JSON-decoded map).
class RradarCapabilities {
  const RradarCapabilities({
    required this.productId,
    required this.version,
    required this.ledgerSchema,
    required this.cloudSync,
    required this.officialRelay,
    required this.multiDeviceHandoff,
    required this.rulePacks,
    required this.localHttpServe,
    required this.tagsAttachments,
    required this.attachmentStore,
    required this.backupIncludesAttachments,
    required this.captureOneshot,
    required this.engineAuto,
    required this.tagFilter,
    required this.localBudgets,
    required this.notes,
  });

  final String productId;
  final String version;
  final int ledgerSchema;
  final bool cloudSync;
  final bool officialRelay;
  final bool multiDeviceHandoff;
  final bool rulePacks;
  final bool localHttpServe;
  final bool tagsAttachments;
  final bool attachmentStore;
  final bool backupIncludesAttachments;
  final bool captureOneshot;
  final bool engineAuto;
  final bool tagFilter;
  final bool localBudgets;
  final String notes;

  factory RradarCapabilities.fromJson(Map<String, dynamic> j) {
    return RradarCapabilities(
      productId: j['product_id'] as String? ?? 'receiptradar',
      version: j['version'] as String? ?? '0.0.0',
      ledgerSchema: (j['ledger_schema'] as num?)?.toInt() ?? 0,
      cloudSync: j['cloud_sync'] as bool? ?? false,
      officialRelay: j['official_relay'] as bool? ?? false,
      multiDeviceHandoff: j['multi_device_handoff'] as bool? ?? false,
      rulePacks: j['rule_packs'] as bool? ?? false,
      localHttpServe: j['local_http_serve'] as bool? ?? false,
      tagsAttachments: j['tags_attachments'] as bool? ?? false,
      attachmentStore: j['attachment_store'] as bool? ?? false,
      backupIncludesAttachments:
          j['backup_includes_attachments'] as bool? ?? false,
      captureOneshot: j['capture_oneshot'] as bool? ?? false,
      engineAuto: j['engine_auto'] as bool? ?? false,
      tagFilter: j['tag_filter'] as bool? ?? false,
      localBudgets: j['local_budgets'] as bool? ?? false,
      notes: j['notes'] as String? ?? '',
    );
  }
}

/// Facade used by screens — never call FRB directly from widgets.
abstract class RradarApi {
  Future<String> apiVersion();
  Future<RradarCapabilities> capabilities();
  Future<List<String>> categories();
  Future<String> defaultLedgerPath();
  Future<String> defaultInboxPath();
  Future<String> defaultRulesPath();

  /// Process a filesystem path; returns draft JSON string.
  Future<String> processPath({
    required String path,
    String currency = 'TWD',
    String engine = 'mock',
    String? qrPayload,
  });

  /// Capture one-shot: process → confirm → optional attach/tags (FFI contract).
  Future<String> processConfirmPath({
    required String dbPath,
    required String path,
    bool confirm = true,
    bool attach = true,
    String tags = 'capture',
    String currency = 'TWD',
    String engine = 'mock',
  });

  Future<void> ensureLedger(String dbPath);
  Future<int> count(String dbPath);
  Future<String> listJson(String dbPath, {int limit = 50});
  Future<String> queryJson(
    String dbPath, {
    int limit = 50,
    String? tag,
    String? category,
    String? query,
  });
  Future<String> listTagsJson(String dbPath);
  Future<String> budgetStatusJson(
    String dbPath, {
    int year = 2024,
    int month = 5,
  });
  Future<String> enginesJson();
  Future<String> statsAllJson(String dbPath);
  Future<String> reportMonthMarkdown(String dbPath, {int year = 2024, int month = 5});
  Future<String> listRulePacksJson();
  Future<String> modelsPinsJson({String modelsDir = ''});
}

/// Global mock used by shell until FRB is generated.
final RradarApi rradarApi = MockRradarApi();

/// In-process mock so UI builds without native libs.
class MockRradarApi implements RradarApi {
  final List<Map<String, dynamic>> _tx = [];
  final Map<String, int> _budgetMinor = {'TWD': 3000000}; // 30000 major * 100

  @override
  Future<String> apiVersion() async => 'receiptradar ffi mock 0.1.0-alpha.0';

  @override
  Future<RradarCapabilities> capabilities() async {
    return const RradarCapabilities(
      productId: 'receiptradar',
      version: '0.1.0-alpha.0',
      ledgerSchema: 3,
      cloudSync: false,
      officialRelay: false,
      multiDeviceHandoff: true,
      rulePacks: true,
      localHttpServe: true,
      tagsAttachments: true,
      attachmentStore: true,
      backupIncludesAttachments: true,
      captureOneshot: true,
      engineAuto: true,
      tagFilter: true,
      localBudgets: true,
      notes: 'mock api — local-first; multi-device via backup/handoff file only',
    );
  }

  @override
  Future<List<String>> categories() async => const [
        'food_dining',
        'grocery_convenience',
        'transport',
        'shopping',
        'health',
        'utilities',
        'entertainment',
        'other',
      ];

  @override
  Future<String> defaultLedgerPath() async => 'mock://receiptradar/ledger.db';

  @override
  Future<String> defaultInboxPath() async => 'mock://receiptradar/inbox';

  @override
  Future<String> defaultRulesPath() async => 'mock://receiptradar/rules';

  @override
  Future<String> processPath({
    required String path,
    String currency = 'TWD',
    String engine = 'mock',
    String? qrPayload,
  }) async {
    final id = 'mock-${_tx.length + 1}';
    _tx.insert(0, {
      'id': id,
      'merchant': path.split(RegExp(r'[/\\]')).last,
      'amount_minor': 8900,
      'currency': currency,
      'category': 'grocery_convenience',
      'tags': 'mock',
    });
    return '{"id":"$id","merchant":{"value":"MOCK","confidence":1.0},'
        '"total":{"value":{"amount_minor":8900,"currency":"$currency"}},'
        '"source_path":"ocr","path":"$path","engine":"$engine",'
        '"qr":${qrPayload == null ? 'null' : '"$qrPayload"'}}';
  }

  @override
  Future<String> processConfirmPath({
    required String dbPath,
    required String path,
    bool confirm = true,
    bool attach = true,
    String tags = 'capture',
    String currency = 'TWD',
    String engine = 'mock',
  }) async {
    final draft = await processPath(
      path: path,
      currency: currency,
      engine: engine,
    );
    if (!confirm) {
      return '{"draft":$draft,"confirmed":false}';
    }
    final id = _tx.isEmpty ? 'mock-1' : _tx.first['id'] as String;
    if (attach) {
      _tx.first['attachment_path'] = 'attachments/$id/capture.jpg';
    }
    _tx.first['tags'] = tags;
    return '{"draft":$draft,"confirmed":true,"inserted":true,'
        '"transaction":{"id":"$id","attachment_path":"${attach ? 'attachments/$id/capture.jpg' : ''}",'
        '"tags":"$tags","amount_minor":8900,"currency":"$currency"}}';
  }

  @override
  Future<void> ensureLedger(String dbPath) async {}

  @override
  Future<int> count(String dbPath) async => _tx.length;

  @override
  Future<String> listJson(String dbPath, {int limit = 50}) async {
    return queryJson(dbPath, limit: limit);
  }

  @override
  Future<String> queryJson(
    String dbPath, {
    int limit = 50,
    String? tag,
    String? category,
    String? query,
  }) async {
    var rows = _tx.toList();
    if (tag != null && tag.isNotEmpty) {
      rows = rows
          .where((t) => (t['tags'] as String? ?? '')
              .split(',')
              .map((s) => s.trim())
              .contains(tag))
          .toList();
    }
    if (category != null && category.isNotEmpty) {
      rows = rows.where((t) => t['category'] == category).toList();
    }
    if (query != null && query.isNotEmpty) {
      final q = query.toLowerCase();
      rows = rows.where((t) {
        final blob =
            '${t['merchant']}${t['category']}${t['tags']}'.toLowerCase();
        return blob.contains(q);
      }).toList();
    }
    final slice = rows.take(limit).toList();
    if (slice.isEmpty) return '[]';
    final parts = slice.map((t) {
      return '{"id":"${t['id']}","merchant":"${t['merchant']}",'
          '"amount_minor":${t['amount_minor']},"currency":"${t['currency']}",'
          '"category":"${t['category']}","tags":"${t['tags']}"}';
    });
    return '[${parts.join(',')}]';
  }

  @override
  Future<String> listTagsJson(String dbPath) async {
    final set = <String>{};
    for (final t in _tx) {
      for (final part in (t['tags'] as String? ?? '').split(',')) {
        final s = part.trim();
        if (s.isNotEmpty) set.add(s);
      }
    }
    if (set.isEmpty) return '[]';
    return '[${set.map((s) => '"$s"').join(',')}]';
  }

  @override
  Future<String> budgetStatusJson(
    String dbPath, {
    int year = 2024,
    int month = 5,
  }) async {
    var spent = 0;
    for (final t in _tx) {
      if (t['currency'] == 'TWD') spent += t['amount_minor'] as int;
    }
    final limit = _budgetMinor['TWD'] ?? 0;
    final over = spent > limit && limit > 0;
    final ratio = limit == 0 ? 0.0 : spent / limit;
    return '[{"currency":"TWD","category":null,"year":$year,"month":$month,'
        '"limit_minor":$limit,"spent_minor":$spent,'
        '"remaining_minor":${limit - spent},"ratio":$ratio,"over":$over}]';
  }

  @override
  Future<String> enginesJson() async =>
      '{"default":"mock","auto_resolves_to":"mock","engines":[{"name":"mock","available":true}]}';

  @override
  Future<String> statsAllJson(String dbPath) async {
    if (_tx.isEmpty) return '[]';
    var sum = 0;
    for (final t in _tx) {
      sum += t['amount_minor'] as int;
    }
    return '[{"currency":"TWD","total_minor":$sum,"count":${_tx.length}}]';
  }

  @override
  Future<String> reportMonthMarkdown(
    String dbPath, {
    int year = 2024,
    int month = 5,
  }) async {
    final n = _tx.length;
    return '# ReceiptRadar report $year-${month.toString().padLeft(2, '0')}\n\n'
        'Mock ledger: **$n** transactions.\n\n'
        '## Budgets\n\nLocal soft limits (no cloud).\n\n'
        'Local-first · no cloud relay.\n';
  }

  @override
  Future<String> listRulePacksJson() async => '[]';

  @override
  Future<String> modelsPinsJson({String modelsDir = ''}) async =>
      '{"dir":"$modelsDir","pins_ok":false,"onnx_feature":false,"pins":[]}';
}

/// Placeholder for generated FRB bindings — throws until wired.
class NativeRradarApi implements RradarApi {
  UnsupportedError get _e => UnsupportedError(
        'NativeRradarApi: generate flutter_rust_bridge bindings (docs/frb-contract.md)',
      );

  @override
  Future<String> apiVersion() async => throw _e;

  @override
  Future<RradarCapabilities> capabilities() async => throw _e;

  @override
  Future<List<String>> categories() async => throw _e;

  @override
  Future<String> defaultLedgerPath() async => throw _e;

  @override
  Future<String> defaultInboxPath() async => throw _e;

  @override
  Future<String> defaultRulesPath() async => throw _e;

  @override
  Future<String> processPath({
    required String path,
    String currency = 'TWD',
    String engine = 'mock',
    String? qrPayload,
  }) async =>
      throw _e;

  @override
  Future<String> processConfirmPath({
    required String dbPath,
    required String path,
    bool confirm = true,
    bool attach = true,
    String tags = 'capture',
    String currency = 'TWD',
    String engine = 'mock',
  }) async =>
      throw _e;

  @override
  Future<void> ensureLedger(String dbPath) async => throw _e;

  @override
  Future<int> count(String dbPath) async => throw _e;

  @override
  Future<String> listJson(String dbPath, {int limit = 50}) async => throw _e;

  @override
  Future<String> queryJson(
    String dbPath, {
    int limit = 50,
    String? tag,
    String? category,
    String? query,
  }) async =>
      throw _e;

  @override
  Future<String> listTagsJson(String dbPath) async => throw _e;

  @override
  Future<String> budgetStatusJson(
    String dbPath, {
    int year = 2024,
    int month = 5,
  }) async =>
      throw _e;

  @override
  Future<String> enginesJson() async => throw _e;

  @override
  Future<String> statsAllJson(String dbPath) async => throw _e;

  @override
  Future<String> reportMonthMarkdown(
    String dbPath, {
    int year = 2024,
    int month = 5,
  }) async =>
      throw _e;

  @override
  Future<String> listRulePacksJson() async => throw _e;

  @override
  Future<String> modelsPinsJson({String modelsDir = ''}) async => throw _e;
}
