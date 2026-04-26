//! SAF layer — outbound public facade.

mod builder;

pub use crate::api::builder::{build_memory_database, Builder};
pub use crate::api::config::{ConfigError, EgressConfig, SinkConfig, SinkFormat, SinkType};
pub use crate::api::egress_error::{EgressError, EgressErrorCode, EgressResult, ResultEgressExt};
pub use crate::api::health_check::{HealthCheck, HealthStatus};
pub use crate::api::outbound_sink::OutputSink;
pub use crate::api::pagination::{PaginatedResponse, Pagination};
pub use crate::api::retry::{BackoffStrategy, RetryConfig};
pub use crate::api::traits::Validator;
pub use crate::provider::{LazyInit, LazyInitWithConfig, StatefulProvider, StatelessProvider};
pub use crate::state::{CachedService, ConfiguredCache};
pub use builder::{
    console_notifier, memory_database, mock_payment_gateway, passthrough_validator, stdout_sink,
};

// Domain crate re-exports
pub use swe_edge_egress_http::{
    FormPart, HttpAuth, HttpBody, HttpConfig, HttpMethod, HttpOutbound, HttpOutboundError,
    HttpOutboundResult, HttpRequest, HttpResponse,
};
pub use swe_edge_egress_grpc::{
    GrpcMetadata, GrpcOutbound, GrpcRequest, GrpcResponse, GrpcStatusCode,
};
pub use swe_edge_egress_database::{
    DatabaseConfig, DatabaseGateway, DatabaseRead, DatabaseType, DatabaseWrite, IsolationLevel,
    QueryParams, Record, WriteResult,
};
pub use swe_edge_egress_file::{
    FileInfo, FileMetadata, FileOutbound, FileStorageConfig, FileStorageType, ListOptions,
    ListResult, PresignedUrl, UploadOptions,
};
pub use swe_edge_egress_notification::{
    EmailConfig, Notification, NotificationChannel, NotificationConfig, NotificationPriority,
    NotificationReceipt, NotificationSender, NotificationStatus, PushConfig, SmsConfig,
    WebhookConfig,
};
pub use swe_edge_egress_payment::{
    CardDetails, Currency, Customer, Money, Payment, PaymentConfig, PaymentGateway, PaymentInbound,
    PaymentMethod, PaymentMethodType, PaymentOutbound, PaymentProvider, PaymentResult,
    PaymentStatus, Refund, RefundReason, RefundResult, RefundStatus,
};
