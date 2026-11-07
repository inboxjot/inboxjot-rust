mod campaigns;
mod client;
mod contacts;
mod domains;
mod emails;
mod error;
mod events;
mod images;
mod segments;
mod shared;
mod templates;
mod topics;
mod webhook_endpoints;

#[cfg(feature = "webhooks")]
pub mod webhooks;

pub use self::campaigns::{
  Campaign, CampaignInput, CampaignListItem, CampaignPage, CampaignPauseReason,
  CampaignSort, CampaignStatus, CampaignSummary, Campaigns, ListCampaigns, Named,
  QueuedIds, SendCampaign, SendTestCampaign,
};
pub use self::client::{Inboxjot, InboxjotBuilder};
pub use self::contacts::{
  Contact, ContactListItem, ContactListPage, ContactSort, ContactStatus, ContactTopic,
  Contacts, CreateContact, EraseContact, ErasureStatus, LastActivity, ListContacts,
  QueuedErasure, SavedContact, SegmentRef, TopicChoice, TopicStatus, UpdateContact,
};
pub use self::domains::{
  CreateDomain, Domain, DomainListItem, DomainPage, DomainReceiving, DomainRecord,
  DomainStatus, DomainWarmup, Domains, RecordKind, TrackingStatus,
};
pub use self::emails::{
  Attachment, AttachmentInput, AttachmentList, BatchEmail, CanceledEmail, Email,
  EmailPage, EmailSort, EmailSource, EmailStatus, EmailStatusOp, Emails, ListEmails,
  QueuedBatch, QueuedEmail, RescheduledEmail, SendEmail, Tag, TemplateRef, UpdateEmail,
};
pub use self::error::{ApiError, Error, ErrorItem, ErrorName, Result};
pub use self::events::{Event, Events, TrackedEvent};
pub use self::images::{Image, ImagePage, ImageType, Images, ListImages, UploadImage};
pub use self::segments::{
  AddedMembers, CreateSegment, ListSegmentMembers, ListSegments, RemovedMembers, Segment,
  SegmentConditions, SegmentListItem, SegmentMatch, SegmentMember, SegmentMemberPage,
  SegmentMembers, SegmentOperator, SegmentPage, SegmentRule, SegmentSort, SegmentType,
  Segments, UpdateSegment,
};
pub use self::shared::{
  CreateVersion, DocumentVersion, DocumentVersionDetail, DocumentVersionEditor,
  DocumentVersionEditorKind, DocumentVersionPage, DocumentVersionReason, ListVersions,
  PageParams, SortDirection, StatusOk, UpdateVersion, VersionFilter,
};
pub use self::templates::{
  CreateTemplate, ListTemplates, Template, TemplateListItem, TemplatePage, TemplateSort,
  TemplateStats, Templates, UpdateTemplate, UsedCampaign, UsedIn,
};
pub use self::topics::{
  CreateTopic, Topic, TopicCounts, TopicPage, TopicVisibility, Topics, UpdateTopic,
};
pub use self::webhook_endpoints::{
  CreateWebhookEndpoint, CreatedWebhookEndpoint, EndpointStatus, UpdateWebhookEndpoint,
  WebhookDelivery, WebhookEndpoint, WebhookEndpointDetail, WebhookEndpointListItem,
  WebhookEndpointPage, WebhookEventType, Webhooks,
};
