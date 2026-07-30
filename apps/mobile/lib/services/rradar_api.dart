/// Abstract ReceiptRadar core API for Flutter UI.
///
/// Mirrors `crates/rradar-ffi` free functions. Until flutter_rust_bridge
/// generates native bindings, [MockRradarApi] drives the shell UI.
library;

/// Capability blob (JSON-decoded map).
class RradarCapabilities {
  const RradarCapabilities({
    required this.productId,
    required this.version,
    required this.ledgerSchema,
    required this.cloudSync,
    required this.officialRelay,
    required this.notes,
  });

  final String productId;
  final String version;
  final int ledgerSchema;
  final bool cloudSync;
  final bool officialRelay;
  final String notes;

  factory RradarCapabilities.fromJson(Map<String, dynamic> j) {
    return RradarCapabilities(
      productId: j['product_id'] as String? ?? 'receiptradar',
      version: j['version'] as String? ?? '0.0.0',
      ledgerSchema: (j['ledger_schema'] as num?)?.toInt() ?? 0,
      cloudSync: j['cloud_sync'] as bool? ?? false,
      officialRelay: j['official_relay'] as bool? ?? false,
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

  /// Process a filesystem path; returns draft JSON string.
  Future<String> processPath({
    required String path,
    String currency = 'TWD',
    String engine = 'mock',
    String? qrPayload,
  });

  Future<void> ensureLedger(String dbPath);
  Future<int> count(String dbPath);
  Future<String> listJson(String dbPath, {int limit = 50});
  Future<String> statsAllJson(String dbPath);
}

/// In-process mock so UI builds without native libs.
class MockRradarApi implements RradarApi {
  @override
  Future<String> apiVersion() async => 'receiptradar ffi mock 0.1.0-alpha.0';

  @override
  Future<RradarCapabilities> capabilities() async {
    return const RradarCapabilities(
      productId: 'receiptradar',
      version: '0.1.0-alpha.0',
      ledgerSchema: 2,
      cloudSync: false,
      officialRelay: false,
      notes: 'mock api — local-first; multi-device via backup file only',
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
  Future<String> defaultLedgerPath() async =>
      'mock://receiptradar/ledger.db';

  @override
  Future<String> processPath({
    required String path,
    String currency = 'TWD',
    String engine = 'mock',
    String? qrPayload,
  }) async {
    return '{"id":"mock","merchant":{"value":"MOCK","confidence":1.0},'
        '"total":{"value":{"amount_minor":0,"currency":"$currency"}},'
        '"source_path":"ocr","path":"$path","engine":"$engine",'
        '"qr":${qrPayload == null ? 'null' : '"$qrPayload"'}}';
  }

  @override
  Future<void> ensureLedger(String dbPath) async {}

  @override
  Future<int> count(String dbPath) async => 0;

  @override
  Future<String> listJson(String dbPath, {int limit = 50}) async => '[]';

  @override
  Future<String> statsAllJson(String dbPath) async => '[]';
}

/// Placeholder for generated FRB bindings — throws until wired.
class NativeRradarApi implements RradarApi {
  @override
  Future<String> apiVersion() async {
    throw UnsupportedError(
      'NativeRradarApi: generate flutter_rust_bridge bindings (docs/ffi.md)',
    );
  }

  @override
  Future<RradarCapabilities> capabilities() =>
      Future.error(UnsupportedError('NativeRradarApi not linked'));

  @override
  Future<List<String>> categories() =>
      Future.error(UnsupportedError('NativeRradarApi not linked'));

  @override
  Future<String> defaultLedgerPath() =>
      Future.error(UnsupportedError('NativeRradarApi not linked'));

  @override
  Future<String> processPath({
    required String path,
    String currency = 'TWD',
    String engine = 'mock',
    String? qrPayload,
  }) =>
      Future.error(UnsupportedError('NativeRradarApi not linked'));

  @override
  Future<void> ensureLedger(String dbPath) =>
      Future.error(UnsupportedError('NativeRradarApi not linked'));

  @override
  Future<int> count(String dbPath) =>
      Future.error(UnsupportedError('NativeRradarApi not linked'));

  @override
  Future<String> listJson(String dbPath, {int limit = 50}) =>
      Future.error(UnsupportedError('NativeRradarApi not linked'));

  @override
  Future<String> statsAllJson(String dbPath) =>
      Future.error(UnsupportedError('NativeRradarApi not linked'));
}

/// App-wide default until DI / FRB lands.
RradarApi rradarApi = MockRradarApi();
