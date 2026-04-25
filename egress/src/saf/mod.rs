//! SAF layer — outbound public facade.

mod builder;

pub use crate::api::builder::{build_memory_database, Builder};
pub use crate::api::config::{ConfigError, EgressConfig, SinkConfig, SinkFormat, SinkType};
pub use crate::api::database::{
    DatabaseConfig, DatabaseGateway, DatabaseRead, DatabaseType, DatabaseWrite,
    IsolationLevel, QueryParams, Record, WriteResult,
};
pub use crate::api::egress_error::{EgressError, EgressErrorCode, EgressResult, ResultEgressExt};
pub use crate::api::file::{
    FileInfo, FileMetadata, FileStorageConfig, FileStorageType, ListOptions, ListResult,
    PresignedUrl, UploadOptions,
};
pub use crate::api::file_outbound::FileOutbound;
pub use crate::api::grpc::{GrpcMetadata, GrpcRequest, GrpcResponse, GrpcStatusCode};
pub use crate::api::grpc_outbound::GrpcOutbound;
pub use crate::api::health_check::{HealthCheck, HealthStatus};
pub use crate::api::http::{HttpAuth, HttpBody, HttpConfig, HttpMethod, HttpRequest, HttpResponse};
pub use crate::api::http_outbound::HttpOutbound;
pub use crate::api::notification::{
    EmailConfig, Notification, NotificationChannel, NotificationConfig, NotificationPriority,
    NotificationReceipt, NotificationStatus, PushConfig, SmsConfig, WebhookConfig,
};
pub use crate::api::notification_sender::NotificationSender;
pub use crate::api::outbound_sink::OutputSink;
pub use crate::api::pagination::{PaginatedResponse, Pagination};
pub use crate::api::payment::{
    CardDetails, Currency, Customer, Money, Payment, PaymentConfig, PaymentMethod,
    PaymentMethodType, PaymentProvider, PaymentResult, PaymentStatus, Refund, RefundReason,
    RefundResult, RefundStatus,
};
pub use crate::api::payment_gateway::{PaymentGateway, PaymentInbound, PaymentOutbound};
pub use crate::api::retry::{BackoffStrategy, RetryConfig};
pub use crate::api::traits::Validator;
pub use crate::provider::{LazyInit, LazyInitWithConfig, StatefulProvider, StatelessProvider};
pub use crate::state::{CachedService, ConfiguredCache};
pub use builder::{
    console_notifier, memory_database, mock_payment_gateway, passthrough_validator, stdout_sink,
};
